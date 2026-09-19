"""Parameter constants. Was `param.rs` before.
"""

import logging
import os

import numpy as np
import torch

"""
    Parameters of thrl.
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

# === Live adjustment after training started === #
RL_BY_HUMAN = False
RESET_OPTIMIZER_ON_RESUME = True
RESET_LAMBDA_ON_RESUME = False

# === PATH === #
LOG_DIR = "./logs"
EXPORT_DIR = "./export"
CFG_FILE = "KAIKII.CFG"
MODEL_SAVE_DIR = "./models"
os.makedirs(LOG_DIR, exist_ok=True)
os.makedirs(EXPORT_DIR, exist_ok=True)
os.makedirs(MODEL_SAVE_DIR, exist_ok=True)

# === RL HYPERPARAMETERS === #
LEARNING_RATE = 1e-4
GAMMA = 0.995
LAMBDA_GAE = 0.98
CLIP_EPSILON = 0.2
VALUE_COEFF = 0.5
ENTROPY_COEFF_START = 0.05
ENTROPY_COEFF_END = 0.01
ENTROPY_ANNEAL_UPDATES = 5000
PPO_EPOCHS = 3
HUMAN_BC_EPOCHS = 3
HUMAN_ADV_WEIGHT_BETA = 0.5
HUMAN_ADV_WEIGHT_MIN = 0.25
HUMAN_ADV_WEIGHT_MAX = 4.0
HORIZON = 2048
ACTION_DIM = 19
LOGIT_CLAMP = 80.0
FRAME_INTERVAL_MS = 36
MAX_GRAD_NORM = 0.5

# === WORKER SETTINGS === #
NUM_WORKERS = 1 if RL_BY_HUMAN else 4
CHUNK_SIZE = 64  # Need to dividable by `SEQ_LEN`
OFF_POLICY = 0
MAX_POLICY_STALENESS = 2

# === MODEL SETTINGS === #
FEATURE_PROJECT_DIM = 128
CNN_EMBED_DIM = 128
CNN_POOL_OUT = (6, 6)
CNN_HIDDEN_CHANNELS = [32, 64, 64]
HIDDEN_DIM = 512
GRU_HIDDEN_SIZE = 256
NUM_OBJECTIVES = 3  # survival, combat, resource

# === GRU === #
SEQ_LEN = 16
NUM_SEQS = HORIZON // SEQ_LEN
SEQS_PER_BATCH = 4

# === OBS SCHEMA === # TODO: Select game. See src/observation/schema{num} and src/games/mod.rs
GRID_W = 96
GRID_H = 92
FEATURE_DIM = 273
MAP_CHANNELS = 24

# [allow(unused)]
GAME = 5

# === REWARD === # TODO: do it in rust. This is test and experimental, and I need quick changes. Compiling and wait the `uv` to finish managing packages are such a pain
REWARD_SCALES = np.array([0.01, 0.001, 0.01], dtype=np.float32)

# === LAGRANGIAN SETTINGS === #
MAX_DEATH_RATE = 0.002
DUAL_LR = 1.0
DUAL_MAX = 3.0
DUAL_MIN = 0.01
INITIAL_LAMBDA = 1.0

# === CHARACTER & CONFIG STUFFS === #
"""
Documentation:
// specific cfg gen (stg cc)

[
  {
    "when": "start",
    "type": "struct",
    "name": "Cfg05",
    "live": 3,
    "bomb": 3,
    "stg": 0,
    "phase": 0,
    "end": 0,
    "cha": 2,
    "rank": 0,
    "power": 0
  },
  {
    "when": "success",
    "type": "advance",
    "name": "specific_cfg_gen"
  }
]

[
  {
    "when": "start",
    "type": "struct",
    "name": "Cfg05",
    "live": 3,
    "bomb": 3,
    "stg": 0,
    "phase": 0,
    "end": 0,
    "cha": 2,
    "rank": 0,
    "power": 0
  }
]

see playperf algorithm in paper of readme
[
  {
    "when": "start",
    "type": "fn",
    "name": "playperf",
    "score": 21,
    "tolerance": 3,
    "time_ms": 500,
    "char_pool": [0, 1, 2, 3]
  },
  {
    "when": "success",
    "type": "fn",
    "name": "playperf",
    "score": {"from": "current_config", "delta": 3},
    "tolerance": 3,
    "time_ms": 500,
    "char_pool": [0, 1, 2, 3]
  },
  {
    "when": "fail",
    "type": "fn",
    "name": "playperf",
    "score": {"from": "current_config", "delta": -3},
    "tolerance": 3,
    "time_ms": 500,
    "char_pool": [0, 1, 2, 3]
  }
]
"""

CURRICULUM_JSON = r"""
[
  {
    "when": "start",
    "type": "struct",
    "name": "Cfg05",
    "live": 3,
    "bomb": 3,
    "stg": 0,
    "phase": 0,
    "end": 0,
    "cha": 2,
    "rank": 0,
    "power": 0
  },
  {
    "when": "success",
    "type": "advance",
    "name": "specific_cfg_gen"
  }
]
"""
CURRICULUM_STATE_FILE = os.path.join(LOG_DIR, "curriculum_state.json")

# my favourite f***ing xpu. I love it so much.
DEVICE = torch.device(
    "xpu"
    if hasattr(torch, "xpu") and torch.xpu.is_available()
    else ("cuda" if torch.cuda.is_available() else "cpu")
)
logging.info(f"moppo: Using {DEVICE}")
