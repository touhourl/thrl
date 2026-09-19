"""Curriculum (CC 1.0) th05 helpers"""

import json
import os

import rrr

from .param import CFG_FILE, CURRICULUM_JSON, EXPORT_DIR

"""
    Curriculum Helpers of thrl.
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


CFG_FIELDS = ("live", "bomb", "stg", "phase", "end", "cha", "rank", "power")


def _execute(event, cfg=None):
    """Inner helper.

    :param event: The event, e.g. fail, success
    :param cfg: The config
    :return: dict
    """
    current_json = json.dumps(cfg_to_dict(cfg), sort_keys=True) if cfg is not None else None
    return json.loads(rrr.cfg_execute(CURRICULUM_JSON, event, current_json))


def cfg_to_dict(cfg):
    """copy dict

    :param cfg
    :return: dic
    """
    return {field: int(cfg[field]) for field in CFG_FIELDS}


def cfg_from_dict(cfg):
    """Reconstruct a configuration from serialized fields.

    :param cfg: if new run.
    :return: Reconstructed configuration, or make_init_cfg()
    """
    if not cfg:
        return make_init_cfg()
    return cfg_to_dict(cfg)


def cfg_debug(cfg):
    """For print.

    :param cfg: rrr.Cfg05
    :return str
    """
    return (f"cfg05{{lives:{cfg['live']}, bombs:{cfg['bomb']}, stage:{cfg['stg']}, "
            f"phase:{cfg['phase']}, end:{cfg['end']}, char:{cfg['cha']}, "
            f"rank:{cfg['rank']}, power:{cfg['power']}}}")


def write_cfg(cfg) -> None:
    """Write the active json config through rs

    :param cfg: Cfg
    """
    rrr.cfg_write_json(
        os.path.join(EXPORT_DIR, CFG_FILE),
        json.dumps(cfg_to_dict(cfg), sort_keys=True),
    )


def make_init_cfg():
    """Create the start configuration.

    :param: None
    :return: dict
    """
    return _execute(event="start")


def advance_cfg(cfg, event):
    """Apply a success/fail cc event

    :param cfg: The cfg defined in param.rs
    :param event: Event to trigger.
    """
    return _execute(event=event, cfg=cfg)
