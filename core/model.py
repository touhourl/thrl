"""
ActorCritic model for RL training. MOPPO (so-called, it should be another name, but it is really is if naive).

With: MapEncoder: 3 * Conv2d(stride=2) => AdaptiveMaxPool2d(6,6) => FC(128)

Features from linear ReLU and from Some(i64) to 128. Documentation are in paper and rust.

GRU: 1-layer ???->256 for temporal memory across frames, so it can really see how bullet moving (we have speed
but tho we need some)

3 Critics currently. No more...

Anyway, welcome to see my paper. I did not mention because in the future I can mention them in docs and paper.
If I forgot to do so, pr or issue it.
"""

import torch
import torch.nn as nn

"""
    Model of thrl.
    Copyright (C) 2026  T. Liu (touhourl@proton.me) and contributors of thrl project

    This program is free software: you can redistribute it and/or modify
    it under the terms of the GNU General Public License as published by
    the Free Software Foundation, either version 3 of the License, or
    (at your option) any later version.

    This program is distributed in the hope that it will be useful,
    but WITHOUT ANY WARRANTY; without even the implied warranty of
    MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
    GNU General Public License for more details.

    You should have received a copy of the GNU General Public License
    along with this program.  If not, see <https://www.gnu.org/licenses/>.
"""

from .param import (
    FEATURE_PROJECT_DIM,
    CNN_EMBED_DIM,
    CNN_POOL_OUT,
    CNN_HIDDEN_CHANNELS,
    HIDDEN_DIM,
    GRU_HIDDEN_SIZE,
    NUM_OBJECTIVES
)


class MapEncoder(nn.Module):
    def __init__(self, in_channels=24, hidden_channels=None, embed_dim=CNN_EMBED_DIM,
                 pool_out=CNN_POOL_OUT):
        super().__init__()
        if hidden_channels is None:
            hidden_channels = CNN_HIDDEN_CHANNELS
        self.conv1 = nn.Conv2d(in_channels, hidden_channels[0], kernel_size=3, stride=2, padding=1)
        self.conv2 = nn.Conv2d(hidden_channels[0], hidden_channels[1], kernel_size=3, stride=2, padding=1)
        self.conv3 = nn.Conv2d(hidden_channels[1], hidden_channels[2], kernel_size=3, stride=2, padding=1)
        # Because there are rarely an enemy in a place, we use max pooling.
        # E.g. If a boss is active there is rarely enemies, vise versa.
        self.pool = nn.AdaptiveMaxPool2d(pool_out)
        fc_in = hidden_channels[2] * pool_out[0] * pool_out[1]
        self.fc = nn.Linear(fc_in, embed_dim)
        self.relu = nn.ReLU()

    def forward(self, maps):
        """Run the map encoder forward pass.

        :param maps: [batch, channels, height, width]
        :return: [batch, embed_dim].
        """
        x = self.relu(self.conv1(maps))
        x = self.relu(self.conv2(x))
        x = self.relu(self.conv3(x))
        x = self.pool(x)
        x = torch.flatten(x, start_dim=1)
        x = self.fc(x)
        return x


class MOActorCritic(nn.Module):
    def __init__(self, feature_dim=273, map_channels=24, embed_dim=CNN_EMBED_DIM,
                 hidden_dim=HIDDEN_DIM, action_dim=19, feature_proj_dim=FEATURE_PROJECT_DIM,
                 gru_hidden_size=GRU_HIDDEN_SIZE, num_objectives=NUM_OBJECTIVES):
        super().__init__()
        self.gru_hidden_size = gru_hidden_size
        self.num_objectives = num_objectives
        self.map_encoder = MapEncoder(in_channels=map_channels, embed_dim=embed_dim)
        self.feature_proj = nn.Linear(feature_dim, feature_proj_dim)

        combined_dim = feature_proj_dim + embed_dim  # 128 + 128 = 256

        # GRU for temporal memory. Even with one frame it can see if bullet is near or far.
        self.gru = nn.GRU(input_size=combined_dim, hidden_size=gru_hidden_size,
                          num_layers=1, batch_first=False)

        # Actor chunk
        self.actor_trunk1 = nn.Linear(gru_hidden_size, hidden_dim)
        self.actor_trunk2 = nn.Linear(hidden_dim, hidden_dim)
        self.actor_head = nn.Linear(hidden_dim, action_dim)
        # Note that for action we still need softmax

        # So here the Modulelist is better because params are included
        self.critic_trunks = nn.ModuleList()
        self.critic_heads = nn.ModuleList()
        # for all objectives make a critic. This is what MORL works.
        for _ in range(num_objectives):
            trunk = nn.Sequential(
                nn.Linear(gru_hidden_size, hidden_dim),
                nn.ReLU(),
                nn.Linear(hidden_dim, hidden_dim),
                nn.ReLU(),
            )
            head = nn.Linear(hidden_dim, 1)
            self.critic_trunks.append(trunk)
            self.critic_heads.append(head)

        self.relu = nn.ReLU()

    def _encode(self, features, maps):
        """Linear regression and then RELU.

        :param features: [batch, feature_dim]
        :param maps: [batch, channels, height, width]
        :return: [batch, feature_proj_dim + embed_dim]
        """
        map_embed = self.map_encoder(maps)
        feat_proj = self.relu(self.feature_proj(features))
        return torch.cat([feat_proj, map_embed], dim=1) # Note, this is not addition.

    def _actor(self, gru_out):
        """Compute policy logits from GRU outputs. 2 RELU

        :param gru_out: [batch, gru_hidden_size]
        :return: [batch, action_dim]
        """
        x = self.relu(self.actor_trunk1(gru_out))
        x = self.relu(self.actor_trunk2(x))
        return self.actor_head(x)

    def _critics(self, gru_out):
        """Compute all objective values.

        :param gru_out: [batch, gru_hidden_size]
        :return: [batch, num_objectives]
        """
        vals = []
        # We find pairs here.
        for trunk, head in zip(self.critic_trunks, self.critic_heads):
            vals.append(head(trunk(gru_out)))
        return torch.cat(vals, dim=-1)

    def forward_step(self, features, maps, hidden=None):
        """During rollout collection run one policy/value step

        :param features: [batch, feature_dim]
        :param maps: [batch, channels, height, width]
        :param hidden: [1, batch, gru_hidden_size] || None
        :return: (logits, values, new_hidden)
        """
        combined = self._encode(features, maps)
        if hidden is None:
            hidden = torch.zeros(1, combined.size(0), self.gru_hidden_size, device=combined.device) # Initial state
        gru_out, new_hidden = self.gru(combined.unsqueeze(0), hidden)
        gru_out = gru_out.squeeze(0)
        logits = self._actor(gru_out)
        values = self._critics(gru_out)
        return logits, values, new_hidden

    def forward_sequence(self, features_seq, maps_seq, initial_hidden, dones_seq):
        """BPTT and it is used for MOPPO update. Note that BPTT does not survives new episode.

        :param features_seq: [batch, seq_len, feature_dim]
        :param maps_seq: [batch, seq_len, channels, height, width]
        :param initial_hidden: [1, batch, gru_hidden_size]
        :param dones_seq: [batch, seq_len]
        :return: (logits, values)
        """
        batch, seq_len = features_seq.shape[:2]
        h = initial_hidden
        all_logits = []
        all_values = []

        for t in range(seq_len):
            # Reset hidden state where the prev step was terminal
            if t > 0:
                done_mask = dones_seq[:, t - 1].float().unsqueeze(0).unsqueeze(-1)
                h = h * (1.0 - done_mask)

            combined = self._encode(features_seq[:, t], maps_seq[:, t])
            gru_out, h = self.gru(combined.unsqueeze(0), h)
            gru_out = gru_out.squeeze(0)
            logits = self._actor(gru_out)
            values = self._critics(gru_out)
            all_logits.append(logits)
            all_values.append(values)

        logits = torch.stack(all_logits, dim=1).reshape(-1, all_logits[0].size(-1))
        values = torch.stack(all_values, dim=1).reshape(-1, self.num_objectives)
        return logits, values