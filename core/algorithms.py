"""
Helpers used by the trainer.
"""

import numpy as np

from .param import GAMMA, LAMBDA_GAE

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


def compute_gae(rewards, values, dones, last_value):
    """
    :param R
    :param V
    :param dones
    :param V_{T}
    :return: Tuple `(A_t^{GAE}, returns)`
    """
    # @misc{schulman2018highdimensionalcontinuouscontrolusing,
    #     title={High-Dimensional Continuous Control Using Generalized Advantage Estimation},
    #     author={John Schulman and Philipp Moritz and Sergey Levine and Michael Jordan and Pieter Abbeel},
    #     year={2018},
    #     eprint={1506.02438},
    #     archivePrefix={arXiv},
    #     primaryClass={cs.LG},
    #     url={https://arxiv.org/abs/1506.02438},
    # }
    adv = np.zeros_like(rewards)
    gae = 0.0
    for t in reversed(range(len(rewards))):
        # Notice here is important to use reversed thing of gae.
        # \hata_t^{(1)} &\defeq  \dv_{t} \
        # \hata_t^{(2)} &\defeq \dv_t + \gamma \dv_{t+1} \
        # \hata_t^{(3)} &\defeq \dv_{t} + \gamma \dv_{t+1} + \gamma^2 \dv_{t+2} \
        # Therefore, we can absolutely do:
        # \hata_t^{(3)} &\defeq \dv_{t} + \gamma \hata_{t+1}^{(2)} \
        # \hatalam_t
        # &\defeq (1-\lambda)\lrparen*{ \hata_t^{(1)} + \lambda \hata_t^{(2)} + \lambda^2 \hata_t^{(3)} + \dots  }\nonumber \
        # &= (1-\lambda)\lrparen*{ \dv_t + \lambda (\dv_t + \gamma \dv_{t+1}) + \lambda^2 (\dv_t + \gamma \dv_{t+1} + \gamma^2 \dv_{t+2}) + \dots }\nonumber \
        # &= (1-\lambda)(
        # \dv_t (1 + \lambda + \lambda^2 + \dots)
        # +\gamma \dv_{t+1} (\lambda + \lambda^2 + \lambda^3 + \dots)\nonumber \
        # &\ \ \ \ \  \ \ \ \ \ \ +\gamma^2 \dv_{t+2} (\lambda^2 + \lambda^3 + \lambda^4 + \dots)
        # +\dots)
        # \nonumber \
        # So for dv_t, the coeff is \frac {1} {1-\lambda}. (Because of geometric series, while \lambda is < 1.).
        # so they are all 1 * \lambda ^ k because multiplying the 1-\lambda.
        # Then we separate the first term, factor out one \gamma\lambda, we get
        # \hatalam_t =\delta_t +\gamma\lambda \hatalam_{t + 1}
        # So to know the A_t, we need to know A_{t + 1} first
        nv = last_value if t == len(rewards) - 1 else values[t + 1]
        mask = 1.0 - dones[t]
        delta = rewards[t] + GAMMA * nv * mask - values[t]
        adv[t] = gae = delta + GAMMA * LAMBDA_GAE * mask * gae
    return adv, adv + values


def infer_observation_dims(sample_features, sample_maps, grid_h, grid_w):
    """Calculate thr observation and check if valid.
    :param sample_features
    :param sample_maps
    :param grid_h
    :param grid_w
    :return: Tuple `(feature_dim, map_channels)`.
    :raises ValueError: If the map length is incompatible with the grid.
    """
    feature_dim = len(sample_features)
    cell_count = grid_h * grid_w
    if cell_count == 0 or len(sample_maps) % cell_count != 0:  # reject some invalid maps.
        raise ValueError(
            f"Map size {len(sample_maps)} not divisible by grid {grid_h}x{grid_w}"
        )
    map_channels = len(sample_maps) // cell_count  # this is not a comment in py.
    return feature_dim, map_channels
