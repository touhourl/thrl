"""CSV logging helper.
If someone says too broad exception clause, ignore him. They are not likely in reality.
Also, if you're really trying to use very good matches. I suggest to write them in rust and
use `thiserror` crate. Python is not for serious cmd line or performance, deterministic results.
Defensive programming.
"""

"""
    CSV Logging Helper of thrl.
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
import csv
import json
import logging
import os

import torch

from .cc import cfg_from_dict, cfg_to_dict
from .param import CURRICULUM_STATE_FILE, LOG_DIR


def append_csv_row(path, fieldnames, row) -> None:
    """Append one CSV row
    
    :param path
    :param fieldnames: An ordered list.
    :param row: Mapping containing values for the new row.
    :return: None
    :raises OSError: If migration or append operations fail, which is not likely
    """
    if os.path.exists(path):
        with open(path, newline="") as f:
            reader = csv.DictReader(f)
            old_header = reader.fieldnames or []
            if old_header != fieldnames:
                rows = []
                for r in reader:
                    r.pop(None, None)  # remove invalid entries
                    rows.append(r)
                tmp = path + ".tmp"
                with open(tmp, "w", newline="") as out:
                    w = csv.DictWriter(out, fieldnames=fieldnames)
                    w.writeheader()
                    for r in rows:
                        w.writerow({k: r.get(k, "") for k in fieldnames})
                os.replace(tmp, path)

    exists = os.path.exists(path)
    with open(path, "a", newline="") as f:
        w = csv.DictWriter(f, fieldnames=fieldnames)
        if not exists:
            w.writeheader()
        w.writerow(row)


def csv_max_int(path, column, default=-1):
    """Read the greatest integer value present in a CSV column.
    
    :param path
    :param column: Column whose values are parsed as integers.
    :param default: Return if issue, like -1 or -39 in linux or int main()
    :return: Greatest parsed integer or default (see above)
    """
    if not os.path.exists(path):
        return default
    best = default
    try:
        with open(path, newline="") as f:
            for row in csv.DictReader(f):
                try:
                    best = max(best, int(row[column]))
                except Exception:
                    pass
    except Exception:
        return default
    return best


def json_save(obj, path):
    """Replace a JSON file through a temporary file.
    
    :param obj: jason serializable object.
    :param path
    :return: None.
    :raises OSError: If writing or replacement fails.
    """
    tmp = path + ".tmp"
    with open(tmp, "w") as f:
        json.dump(obj, f, indent=2, sort_keys=True)
    os.replace(tmp, path)


def save_curriculum_state(cfg, episode, update_step):
    """Keep states.
    
    :param cfg: Active `rrr.Cfg05` configuration.
    :param episode: Not used currently
    :param update_step
    :return: None
    """
    json_save({
        "cfg": cfg_to_dict(cfg),
        "episode": int(episode),
        "update_step": int(update_step),
    }, CURRICULUM_STATE_FILE)


def load_curriculum_state(default_cfg):
    """Load cc 1.0 config.
    We might have a list called [314, 276, 361], while the first 2 are normal and only the last one is required.
    Anyway this does not affect me.
    :param default_cfg: Given the default back is non found or success
    :return: Loaded configuration
    """
    if not os.path.exists(CURRICULUM_STATE_FILE):
        return default_cfg
    try:
        with open(CURRICULUM_STATE_FILE) as f:
            state = json.load(f)
        return cfg_from_dict(state.get("cfg"))
    except Exception as e:
        logging.warning(f"Failed to load curriculum state: {e}")
        return default_cfg


def torch_save(obj, path):
    """Save a PyTorch object.
    
    :param obj: Object accepted by `torch.save`.
    :param path
    :return: None
    """
    tmp = path + ".tmp"
    torch.save(obj, tmp)
    os.replace(tmp, path)


# TODO: Save a burn compatible model. I hate non-static typed and dyn pyobjs. So might only docstring as the types, hum.

def get_update_count():
    """Count update rows in the configured update log.
    
    :return: Number of data rows, or zero when bad log data.
    """
    path = os.path.join(LOG_DIR, "updates.csv")
    if not os.path.exists(path):
        return 0
    try:
        with open(path) as f:
            return sum(1 for _ in csv.DictReader(f))
    except Exception:
        return 0


def log_episode(ep, mo_r, t, flag, pp, lam, cfg):
    """Append one completed episode record.
    
    :param ep
    :param mo_r
    :param t
    :param flag: Terminal flag
    :param pp: playperf
    :param lam
    :param cfg
    :return: None
    """
    fn = ["episode", "r_survival", "r_combat", "r_resource",
          "length", "final_state", "playperf", "lambda",
          "cfg_live", "cfg_bomb", "cfg_stage", "cfg_phase", "cfg_end",
          "cfg_char", "cfg_rank", "cfg_power"]
    p = os.path.join(LOG_DIR, "episodes.csv")
    s = {1: "GameOver", 2: "Success", 0: "Running"}.get(flag)  # End of episode
    append_csv_row(p, fn, {
        "episode": ep,
        "r_survival": f"{mo_r[0]:.2f}",
        "r_combat": f"{mo_r[1]:.2f}",
        "r_resource": f"{mo_r[2]:.2f}",
        "length": t,
        "final_state": s,
        "playperf": pp,
        "lambda": f"{lam:.4f}",
        "cfg_live": cfg["live"],
        "cfg_bomb": cfg["bomb"],
        "cfg_stage": cfg["stg"],
        "cfg_phase": cfg["phase"],
        "cfg_end": cfg["end"],
        "cfg_char": cfg["cha"],
        "cfg_rank": cfg["rank"],
        "cfg_power": cfg["power"],
    })


def log_update(ep, step, pl, vl_s, vl_c, vl_r, ent, kl, tl, lam, cost_mean,
               cfg, rollout_successes, rollout_gameovers) -> None:
    """Append one record to csv, but currently, episodes are not counted.
    
    :param ... it is too much.
    :return: None
    """
    fn = ["episode", "update_step", "policy_loss",
          "vl_survival", "vl_combat", "vl_resource",
          "entropy", "approx_kl", "total_loss", "lambda", "cost_mean"]
    fn += ["rollout_successes", "rollout_gameovers",
           "cfg_live", "cfg_bomb", "cfg_stage", "cfg_phase", "cfg_end",
           "cfg_char", "cfg_rank", "cfg_power"]
    p = os.path.join(LOG_DIR, "updates.csv")
    append_csv_row(p, fn, {
        "episode": ep, "update_step": step,
        "policy_loss": pl,
        "vl_survival": vl_s,
        "vl_combat": vl_c,
        "vl_resource": vl_r,
        "entropy": ent,
        "approx_kl": kl,
        "total_loss": tl,
        "lambda": lam,
        "cost_mean": cost_mean,
        "rollout_successes": rollout_successes,
        "rollout_gameovers": rollout_gameovers,
        "cfg_live": cfg["live"],
        "cfg_bomb": cfg["bomb"],
        "cfg_stage": cfg["stg"],
        "cfg_phase": cfg["phase"],
        "cfg_end": cfg["end"],
        "cfg_char": cfg["cha"],
        "cfg_rank": cfg["rank"],
        "cfg_power": cfg["power"],
    })
