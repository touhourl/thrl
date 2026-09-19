"""Rollout worker process for --TH05-- DOSBox-X (?) collection."""

import io
import time

import rrr

from .model import MOActorCritic
from .param import *

"""
    Training Workers of thrl.
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


def worker_fn(worker_id, chunk_queue, pipe, initial_weights_bytes, initial_policy_ver) -> None:
    """Run one rollout collecting subprocess.

    :param worker_id
    :param chunk_queue: Queue for steps and data chunk, which will give to host
    :param pipe: pool, receive
    :param initial_weights_bytes: Init model weights
    :param initial_policy_ver: Policy version
    :return: None
    """
    # Prevent you stupid pytorch spawning fork bombs in my machine!!!
    torch.set_num_threads(1)
    torch.set_num_interop_threads(1)
    rrr.init_logging()
    logging.info(f"w{worker_id}: Starting worker at policy version {initial_policy_ver}")

    # Then, we use our cpu carefully. Cuz at the same time, xpu/cuda are being used
    # and boom card or 200ms sync (which in rust it is like this, Vulkan) are possible.
    model = MOActorCritic(feature_dim=FEATURE_DIM, map_channels=MAP_CHANNELS,
                          action_dim=ACTION_DIM).cpu()
    # Start model, and we don't train it, so eval.
    model.eval()
    # Load init weights of model
    buf = io.BytesIO(initial_weights_bytes)
    state_dict = torch.load(buf, map_location="cpu",
                            weights_only=False)  # We need to here specific the cpu, else where?
    model.load_state_dict(state_dict)
    local_policy_ver = initial_policy_ver
    local_config_ver = 0

    grid_h, grid_w = GRID_H, GRID_W
    n_obj = NUM_OBJECTIVES

    def spawn_watcher():
        w = rrr.MemoryWatcher(spawn_dosbox=True)
        # Wait for first valid state, dosbox-x
        for _ in range(100):  # Cuz the rrr is so dumb and I did not write a wait in rust, just quick do it
            # ... wait the start
            s = w.read_state()
            if s is not None:
                return w, s
            time.sleep(0.2)
        raise RuntimeError("Worker could not get initial state. Is it killed?")

    watcher, state = spawn_watcher()
    hidden = torch.zeros(1, 1, GRU_HIDDEN_SIZE)

    # Tho this is python, not C++, we still need manually "memory management".
    # We preallocate the trajectory buffers, so we do not need to refrequently do it in training loop.
    # Features, Maps, Discrete Actions, Rewards, Terminal States, Log Probabilities, Critic Predictions, Recurrent State
    # Note that, we are not doing rust. So we use f32 so for a better calc.
    c_feat = np.zeros((CHUNK_SIZE, FEATURE_DIM), dtype=np.float32)
    c_maps = np.zeros((CHUNK_SIZE, MAP_CHANNELS, grid_h, grid_w), dtype=np.float32)
    c_acts = np.zeros(CHUNK_SIZE, dtype=np.int64)
    c_rews = np.zeros((CHUNK_SIZE, n_obj), dtype=np.float32)
    c_dones = np.zeros(CHUNK_SIZE, dtype=np.float32)
    c_lp = np.zeros(CHUNK_SIZE, dtype=np.float32)
    c_vals = np.zeros((CHUNK_SIZE, n_obj), dtype=np.float32)
    c_hid = np.zeros((CHUNK_SIZE, GRU_HIDDEN_SIZE), dtype=np.float32)

    # Is total reward, not seperate obj R.
    ep_rewards = np.zeros(n_obj, dtype=np.float64)
    ep_len = 0

    while True:
        successes_in_chunk = 0
        gameovers_in_chunk = 0
        valid_steps = 0

        # We collect exactly Chucksize so many things, so no "Can't keep up! Is the GPU overloading? Running
        # N steps behind". Note, start at zero.
        for step in range(CHUNK_SIZE):
            t0 = time.time()

            if state is None:
                # Lost connection, try to re-read
                for _ in range(50):
                    state = watcher.read_state()
                    if state is not None:
                        break
                    time.sleep(0.1)
                if state is None:
                    break

            f, m, _, _, _ = state  # done_flag, rewards_vec, raw_frame
            # features to numpy f32 array
            f_np = np.array(f, dtype=np.float32)
            m_np = np.array(m, dtype=np.float32).reshape(MAP_CHANNELS, grid_h, grid_w)
            # 24 channels $\times 96 \times 92$ \text{ cells} = [24, 96, 92] \text{ but tensor is not here.}
            f_t = torch.from_numpy(f_np).unsqueeze(0)
            m_t = torch.from_numpy(m_np).unsqueeze(0)
            # Now to tensers. With batch dim.
            c_hid[step] = hidden.squeeze().detach().numpy()
            # this convert to numpy back (hidden) but idk why use detach() here...
            # === POLICY INFERENCE === #
            with torch.no_grad():
                logits, values_mo, new_hidden = model.forward_step(f_t, m_t, hidden)
                logits = torch.clamp(logits, -LOGIT_CLAMP, LOGIT_CLAMP)
                probs = torch.softmax(logits, dim=-1)  # Softmax policy, one of the Policy Gradient Methods

                if RL_BY_HUMAN:
                    # Read the actual action taken by me.
                    action = int(watcher.read_human_action())
                else:
                    # Sample random & Grumbel, but we don't want log 0.
                    u = torch.rand_like(logits)
                    gumbel = -torch.log(-torch.log(u + 1e-10) + 1e-10)
                    # Gumbel-max trick, reference. Got em directly, so no additional proofs.
                    action = (logits + gumbel).argmax(dim=-1).item()
                    # @misc{huijben2022reviewgumbelmaxtrickextensions,
                    # title={A Review of the Gumbel-max Trick and its Extensions for Discrete Stochasticity in Machine Learning},
                    # author={Iris A. M. Huijben and Wouter Kool and Max B. Paulus and Ruud J. G. van Sloun},
                    # year={2022},
                    # eprint={2110.01515},
                    # archivePrefix={arXiv},
                    # primaryClass={cs.LG},
                    # url={https://arxiv.org/abs/2110.01515},
                    # }
                log_prob = torch.log(probs[0, action] + 1e-10).item()
                # = \log \pi_{\theta_{old}}(a_t | s_t, h_t)
                vals_np = values_mo.squeeze(0).numpy()

            hidden = new_hidden

            # This is a similar approach of the rust approach, (I have not taken account for overloading, can't keep up
            # situations) Will find a crate like TODO
            # use std::time::{Instant, SystemTime, UNIX_EPOCH};
            # use tokio::time::{Duration, sleep, interval, MissedTickBehavior};
            # and do frame_timer.tick().await; to skip a tick. I have not written killing the game with -19 and then
            # finished ppo update / finished tick then run -18.
            # In python or move such a critical loop into rust. if burn-rs, all solved... but I still have xpu...
            # Literally zero money to buy a cuda, also idk Computer or Laptop... due to the need of Matrix
            # Multiplication Computing those years. DDR4 or lower will not be acceptable I tho.
            while True:
                watcher.apply_action(action)
                elapsed = (time.time() - t0) * 1000
                if elapsed >= FRAME_INTERVAL_MS:
                    break
                time.sleep(0.005)

            next_state = watcher.read_state()
            if next_state is None:
                state = None
                break

            # Calc R and all transitions (into chunk).
            _, _, next_done_flag, next_rewards, _ = next_state
            r_vec = np.array(next_rewards, dtype=np.float32) * REWARD_SCALES

            c_feat[step] = f_np
            c_maps[step] = m_np
            c_acts[step] = action
            c_rews[step] = r_vec
            c_dones[step] = float(next_done_flag != 0)
            c_lp[step] = log_prob
            c_vals[step] = vals_np
            valid_steps += 1

            ep_rewards += r_vec
            ep_len += 1

            if next_done_flag != 0:
                if next_done_flag == 2:
                    successes_in_chunk += 1
                else:
                    gameovers_in_chunk += 1
                logging.info(f"w{worker_id}: "
                             f"{'Success' if next_done_flag == 2 else 'GameOver'} "
                             f"R=[{ep_rewards[0]:.1f},{ep_rewards[1]:.1f},{ep_rewards[2]:.1f}] "
                             f"L={ep_len}")
                ep_rewards = np.zeros(n_obj, dtype=np.float64)
                ep_len = 0
                hidden = torch.zeros(1, 1, GRU_HIDDEN_SIZE)
                watcher.release_action()
                time.sleep(4.0)  # For safety, we make it a bit longer. Prepare for next one.
                watcher.clear_state()
                next_state = watcher.read_state()
                while next_state is None:
                    time.sleep(0.1)
                    next_state = watcher.read_state()

            state = next_state

        # Send chunk to host
        if valid_steps > 0:
            # Compute bootstrap value for last state
            last_val = np.zeros(n_obj, dtype=np.float32)
            if state is not None and valid_steps == CHUNK_SIZE:
                f, m, _, _, _ = state
                f_t = torch.from_numpy(np.array(f, dtype=np.float32)).unsqueeze(0)
                m_t = torch.from_numpy(np.array(m, dtype=np.float32).reshape(
                    1, MAP_CHANNELS, grid_h, grid_w))
                with torch.no_grad():
                    _, lv, _ = model.forward_step(f_t, m_t, hidden)
                    last_val = lv.squeeze(0).numpy()  # V bootstrap for td.

            chunk = {
                "features": c_feat[:valid_steps].copy(),
                "maps": c_maps[:valid_steps].copy(),
                "actions": c_acts[:valid_steps].copy(),
                "rewards": c_rews[:valid_steps].copy(),
                "dones": c_dones[:valid_steps].copy(),
                "log_probs": c_lp[:valid_steps].copy(),
                "values": c_vals[:valid_steps].copy(),
                "hidden": c_hid[:valid_steps].copy(),
                "last_value": last_val,
                "policy_version": local_policy_ver,
                "config_version": local_config_ver,
                "successes": successes_in_chunk,
                "gameovers": gameovers_in_chunk,
                "worker_id": worker_id,
                "n_steps": valid_steps,
            }
            try:
                chunk_queue.put(chunk)
            except Exception as e:
                logging.info(f"w{worker_id}: Failed to put chunk in queue: {e}, exiting")
                break

        # Check for messages from host
        paused = False
        while paused or pipe.poll():
            msg = pipe.recv()
            msg_type = msg["type"]
            if msg_type == "pause":
                if OFF_POLICY or paused:
                    continue
                watcher.release_action()
                watcher.pause_game()
                paused = True
                pipe.send({
                    "type": "paused",
                    "worker_id": worker_id,
                    "version": local_policy_ver,
                })
            elif msg_type == "weights":
                buf = io.BytesIO(msg["data"])
                sd = torch.load(buf, map_location="cpu", weights_only=False)
                model.load_state_dict(sd)
                local_policy_ver = msg["version"]
                hidden = torch.zeros(1, 1, GRU_HIDDEN_SIZE)
                logging.info(
                    f"w{worker_id}: Policy updated to v{local_policy_ver}"
                )
                if paused:
                    pipe.send({
                        "type": "ready",
                        "worker_id": worker_id,
                        "version": local_policy_ver,
                    })
            elif msg_type == "config":  # future consideration also not used.
                local_config_ver = msg["version"]
                logging.info(
                    f"w{worker_id}: Config v{local_config_ver}, restarting."
                )
                if paused:
                    watcher.resume_game()
                watcher.terminate()
                time.sleep(1.0)
                watcher, state = spawn_watcher()
                if paused:
                    watcher.pause_game()
                hidden = torch.zeros(1, 1, GRU_HIDDEN_SIZE)
                ep_rewards = np.zeros(n_obj, dtype=np.float64)
                ep_len = 0
            elif msg_type == "resume":
                if paused:
                    watcher.resume_game()
                    paused = False
            elif msg_type == "shutdown":  # Should not be like this in current version.
                # deprecated
                if paused:
                    watcher.resume_game()
                logging.info(f"w{worker_id}: Shutdown received")
                watcher.terminate()
                return
