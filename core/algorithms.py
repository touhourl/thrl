import importlib
import json

import numpy as np
import rrr

"""
    Algorithm Helpers of thrl.
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

def compute_gae(rewards, values, dones, last_value, gamma, lambda_gae):
    rewards = np.asarray(rewards, dtype=np.float32)
    values = np.asarray(values, dtype=np.float32)
    dones = np.asarray(dones, dtype=np.float32)
    adv = np.zeros_like(rewards)
    gae = 0.0
    for t in reversed(range(len(rewards))):
        nv = last_value if t == len(rewards) - 1 else values[t + 1]
        mask = 1.0 - dones[t]
        delta = rewards[t] + gamma * nv * mask - values[t]
        gae = delta + gamma * lambda_gae * mask * gae
        adv[t] = gae
    return adv, adv + values


def entropy_coeff(update_step, start, end, anneal_updates):
    if anneal_updates == 0:
        return end
    t = min(update_step / anneal_updates, 1.0)
    return start + t * (end - start)


def train():
    name = json.loads(rrr.runtime_config_json())["runtime"]["algorithm"]
    importlib.import_module(f".{name}", __package__).train()
