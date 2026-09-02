"""Most important trainer.
"""

"""
    Main Trainer of thrl.
    Copyright (C) 2026  T. Liu and contributors

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

import io
import multiprocessing as mp
import queue
import time

import rrr
import torch.nn as nn
import torch.optim as optim

from .algorithms import compute_gae
from .cc import (
    advance_cfg,
    cfg_debug,
    cfg_from_dict,
    cfg_to_dict,
    make_init_cfg,
    write_cfg,
)
from .logging import (
    torch_save,
    csv_max_int,
    load_curriculum_state,
    log_update,
    save_curriculum_state,
)
from .model import MOActorCritic
from .param import *  # this is the best feeling moment. I nearly forgot that `*` is supported.
from .worker import worker_fn

def train() -> None:
    """Main trainer (dummy docstring here)

    :raises ___ don't know, any errors
    :return None
    """
    rrr.init_logging()
    mp.set_start_method("spawn", force=True)  # might not good, will be changed if we have taisei done
    # TODO: Spec the method. Using spawn is a bit off-policy because while host update, worker can be paused
    playperf = 21
    n_obj = NUM_OBJECTIVES
    grid_h, grid_w = GRID_H, GRID_W

    model = MOActorCritic(feature_dim=FEATURE_DIM, map_channels=MAP_CHANNELS,
                          action_dim=ACTION_DIM).to(DEVICE)
    optimizer = optim.Adam(model.parameters(), lr=LEARNING_RATE)
    # @misc{kingma2017adammethodstochasticoptimization,
    #       title={Adam: A Method for Stochastic Optimization},
    #       author={Diederik P. Kingma and Jimmy Ba},
    #       year={2017},
    #       eprint={1412.6980},
    #       archivePrefix={arXiv},
    #       primaryClass={cs.LG},
    #       url={https://arxiv.org/abs/1412.6980},
    # }
    log_lambda = torch.tensor(np.log(INITIAL_LAMBDA), dtype=torch.float32,
                              device=DEVICE, requires_grad=False)

    start_ep, update_step = 0, 0
    cfg = None
    ckpt_path = os.path.join(MODEL_SAVE_DIR, "moppo_latest.pt")

    # See docs for when to reset them. Normally, no need.
    if os.path.exists(ckpt_path):
        logging.info(f"Loading MOPPO checkpoint from {ckpt_path}")
    try:
        ckpt = torch.load(ckpt_path, map_location=DEVICE, weights_only=False)
        model.load_state_dict(ckpt["model_state"])
        if RESET_OPTIMIZER_ON_RESUME:
            logging.warning("Resetting optimizer...")
            pass
        else:
            optimizer.load_state_dict(ckpt["optimizer_state"])
            # Important if loading optimizer state across devices
            for state in optimizer.state.values():
                for k, v in state.items():
                    if torch.is_tensor(v):
                        state[k] = v.to(DEVICE)

        start_ep = ckpt.get("episode", 0)  # I don't wanna remove it. Might get fixed after release.
        playperf = ckpt.get("playperf", 21)
        update_step = ckpt.get("update_step", 0)  # this is accurate anyway

        # reset lambda cuz old checkpoint had lambda pollution (when testing, also normally no need, see docs)
        if RESET_LAMBDA_ON_RESUME:
            lambda_init = float(np.clip(INITIAL_LAMBDA, DUAL_MIN, DUAL_MAX))
            log_lambda = torch.tensor(
                np.log(lambda_init),
                dtype=torch.float32,
                device=DEVICE
            )
            logging.warning(f"Resetting lambda to {lambda_init:.4f}")
        else:
            lambda_loaded = ckpt.get("log_lambda", np.log(INITIAL_LAMBDA))
            lambda_loaded = float(np.clip(np.exp(lambda_loaded), DUAL_MIN, DUAL_MAX))
            log_lambda = torch.tensor(
                np.log(lambda_loaded),
                dtype=torch.float32,
                device=DEVICE
            )

        cfg = cfg_from_dict(ckpt.get("cfg"))

        logging.info(f"Resumed ep={start_ep}, "
                     f"update={update_step}, lambda={torch.exp(log_lambda).item():.4f}"
                     )

    except Exception as e:
        logging.warning(f"Checkpoint load failed: {e}. Starting fresh.")

    cfg = load_curriculum_state(cfg)
    if cfg is None:
        cfg = make_init_cfg()

    last_logged_ep = csv_max_int(os.path.join(LOG_DIR, "episodes.csv"), "episode")
    last_logged_update = csv_max_int(os.path.join(LOG_DIR, "updates.csv"), "update_step")
    start_ep = max(start_ep, last_logged_ep + 1)
    update_step = max(update_step, last_logged_update)

    episode = start_ep
    policy_version = update_step
    config_version = 0

    write_cfg(cfg)
    save_curriculum_state(cfg, episode, update_step)
    logging.info(f"ep={start_ep}, device={DEVICE}, playperf={playperf}, "
                 f"updates={update_step}, lambda={torch.exp(log_lambda).item():.4f}")
    logging.info(f"Curriculum cfg: {cfg_debug(cfg)}")

    # Serialize initial weights for
    # I did not know that python can function in functions. If there is no duplicated code, I have never discovered it.
    def serialize_weights():
        buf = io.BytesIO()
        torch.save(model.cpu().state_dict(), buf)
        model.to(DEVICE)
        return buf.getvalue()
    initial_weights = serialize_weights()

    def wait_for_workers(message_type, version):
        waiting = set(range(NUM_WORKERS))
        while waiting:
            for wid in tuple(waiting):
                if host_pipes[wid].poll(0.01): 
                    # wait for each 10ms which is well under 57.??? fps.
                    msg = host_pipes[wid].recv()
                    if msg.get("type") == message_type and msg.get("version") == version:
                        waiting.remove(wid)
                elif not workers[wid].is_alive():
                    raise RuntimeError(
                        f"Worker {wid} exited"
                    )

    logging.info(f"Preparation Complete. Spawning Workers.")

    workers = []
    host_pipes = []
    chunk_queue = mp.Queue()
    for wid in range(NUM_WORKERS):
        host_end, worker_end = mp.Pipe()
        p = mp.Process(target=worker_fn, args=(wid, chunk_queue, worker_end, initial_weights, policy_version),
                       daemon=True)
        p.start()
        worker_end.close()
        workers.append(p)
        host_pipes.append(host_end)
    logging.info(f"Spawned {NUM_WORKERS} workers")

    # Rollout Chunks from worker (resv) for PPO updates
    acc_features = []
    acc_maps = []
    acc_actions = []
    acc_rewards = []
    acc_dones = []
    acc_log_probs = []
    acc_values = []
    acc_hidden = []
    acc_last_values = []
    acc_steps = 0
    rollout_successes = 0
    rollout_gameovers = 0

    def clear_acc() -> None:
        nonlocal acc_steps, rollout_successes, rollout_gameovers
        acc_features.clear()
        acc_maps.clear()
        acc_actions.clear()
        acc_rewards.clear()
        acc_dones.clear()
        acc_log_probs.clear()
        acc_values.clear()
        acc_hidden.clear()
        acc_last_values.clear()
        acc_steps = 0
        rollout_successes = 0
        rollout_gameovers = 0

    while True:
        # Poll chunk_queue for chunks
        got_any = False
        while True:
            try:
                chunk = chunk_queue.get_nowait()
                got_any = True

                # Reject chunks from old CC config.
                # They may have been produced while I was doing PPO update.
                if chunk["config_version"] != config_version:
                    logging.warning(f"host: Rejected old-config chunk from W{chunk['worker_id']} "
                                 f"(chunk cfg v{chunk['config_version']} vs current cfg v{config_version})"
                                 )
                    continue

                rollout_successes += chunk["successes"]
                rollout_gameovers += chunk["gameovers"]

                # Reject chunks that policy too old.
                staleness = policy_version - chunk["policy_version"]
                # The on policy rejects the chunk anyway so we won't get too many messages in tty.
                if OFF_POLICY and staleness > MAX_POLICY_STALENESS:
                    logging.info(f"host: Rejected stale chunk from W{chunk['worker_id']} "
                                 f"(policy v{chunk['policy_version']} vs current v{policy_version})"
                                 )
                    continue
                
                if not OFF_POLICY and staleness != 0:
                    continue

                ns = chunk["n_steps"]
                acc_features.append(chunk["features"])
                acc_maps.append(chunk["maps"])
                acc_actions.append(chunk["actions"])
                acc_rewards.append(chunk["rewards"])
                acc_dones.append(chunk["dones"])
                acc_log_probs.append(chunk["log_probs"])
                acc_values.append(chunk["values"])
                acc_hidden.append(chunk["hidden"])
                acc_last_values.append(chunk["last_value"])
                acc_steps += ns
            except queue.Empty:
                break

        if acc_steps < HORIZON:
            if not got_any:
                time.sleep(0.01)
            continue

        if not OFF_POLICY:
            for pipe in host_pipes:
                pipe.send({"type": "pause", "version": policy_version})
            wait_for_workers("paused", policy_version)

        # === (MO)PPO Update === #
        logging.info(f"moppo: Update {update_step} ({acc_steps} steps from workers)...")
        t_ppo = time.time()

        # Compute GAE per chunk to avoid boud leaks
        chunk_advs = []
        chunk_rets = []
        for i in range(len(acc_features)):
            c_rew = acc_rewards[i]
            c_val = acc_values[i]
            c_done = acc_dones[i]
            c_lv = acc_last_values[i]

            c_adv = np.zeros_like(c_rew)
            c_ret = np.zeros_like(c_rew)
            for o in range(n_obj):
                adv_o, ret_o = compute_gae(c_rew[:, o], c_val[:, o], c_done, c_lv[o])
                c_adv[:, o] = adv_o
                c_ret[:, o] = ret_o
            chunk_advs.append(c_adv)
            chunk_rets.append(c_ret)

        # Throw all things that exceed Horizon away to keep it up.
        all_advantages = np.concatenate(chunk_advs, axis=0)[:HORIZON]
        all_returns = np.concatenate(chunk_rets, axis=0)[:HORIZON]

        features_buf = np.concatenate(acc_features, axis=0)[:HORIZON]
        maps_buf = np.concatenate(acc_maps, axis=0)[:HORIZON]
        actions_buf = np.concatenate(acc_actions, axis=0)[:HORIZON]
        rewards_buf = np.concatenate(acc_rewards, axis=0)[:HORIZON]
        dones_buf = np.concatenate(acc_dones, axis=0)[:HORIZON]
        log_probs_buf = np.concatenate(acc_log_probs, axis=0)[:HORIZON]
        values_buf = np.concatenate(acc_values, axis=0)[:HORIZON]
        hidden_buf = np.concatenate(acc_hidden, axis=0)[:HORIZON]
        H = len(features_buf)

        # Save rollout success/gameover stats before clearing
        rs = int(rollout_successes)
        rg = int(rollout_gameovers)

        clear_acc()
        # Wasted

        if RL_BY_HUMAN:
            logging.warning("Experimental feature. Ensure you use a DE and used dosbox-x/test.conf")
            # Human training
            # TODO: Experimental feature.
            # The PPO ratio cannot be applied here.
            # For each obj, do std deviation and avoid ner zero devide.
            norm_advantages = np.zeros_like(all_advantages)
            for o in range(n_obj):
                adv_o = all_advantages[:, o]
                std = adv_o.std()
                if std > 1e-5:
                    norm_advantages[:, o] = (adv_o - adv_o.mean()) / (std + 1e-8)
                else:
                    norm_advantages[:, o] = np.zeros_like(adv_o)

            # Combine them but with lambda for survival
            lam_val = torch.exp(log_lambda).item()
            combined_advantages = (
                    norm_advantages[:, 1]
                    + norm_advantages[:, 2]
                    + lam_val * norm_advantages[:, 0]
            )

            # Normalizes the combined advantage. If not useful, we discard it.
            combined_std = combined_advantages.std()
            if combined_std > 1e-5:
                combined_advantages = (combined_advantages - combined_advantages.mean()) / \
                                      (combined_std + 1e-8)
            else:
                combined_advantages = np.zeros_like(combined_advantages)

            # Clipped like PPO so better things get more happened.
            human_weights = np.exp(HUMAN_ADV_WEIGHT_BETA * combined_advantages)
            human_weights = np.clip(
                human_weights,
                HUMAN_ADV_WEIGHT_MIN,
                HUMAN_ADV_WEIGHT_MAX,
            )
            human_weights = human_weights / (human_weights.mean() + 1e-8)

            # If episode-level death, it will happen forever. because it will not pass mostly, so transitions.
            death_rate = (rewards_buf[:, 0] < -0.25).mean()

            # Keeps only data divisible by the seq len
            num_seqs = H // SEQ_LEN
            H_trunc = num_seqs * SEQ_LEN

            # Convert them all to tensors.
            s_feat = torch.from_numpy(features_buf[:H_trunc].reshape(
                num_seqs, SEQ_LEN, FEATURE_DIM).copy()).to(DEVICE)
            s_maps = torch.from_numpy(maps_buf[:H_trunc].reshape(
                num_seqs, SEQ_LEN, MAP_CHANNELS, grid_h, grid_w).copy()).to(DEVICE)
            s_acts = torch.from_numpy(actions_buf[:H_trunc].reshape(
                num_seqs, SEQ_LEN).copy()).to(DEVICE)
            s_ret = torch.from_numpy(all_returns[:H_trunc].reshape(
                num_seqs, SEQ_LEN, n_obj).astype(np.float32)).to(DEVICE)
            s_don = torch.from_numpy(dones_buf[:H_trunc].reshape(
                num_seqs, SEQ_LEN).copy().astype(np.float32)).to(DEVICE)
            s_hid = torch.from_numpy(hidden_buf[:H_trunc:SEQ_LEN].copy()).to(DEVICE)
            s_wgt = torch.from_numpy(human_weights[:H_trunc].reshape(
                num_seqs, SEQ_LEN).astype(np.float32)).to(DEVICE)

            # I never know how to get a person's entropy, but for logging it is required right?
            # Update initialization
            entropy_coeff = 0.0
            seqs_per_batch = min(SEQS_PER_BATCH, num_seqs)
            tot_pl, tot_ent, tot_kl, tot_loss, n_upd = 0, 0, 0, 0, 0
            tot_vl = np.zeros(n_obj, dtype=np.float64)

            # "HMPPO" update
            for _ in range(HUMAN_BC_EPOCHS):
                perm = torch.randperm(num_seqs)
                for bi in range(0, num_seqs, seqs_per_batch):
                    idx = perm[bi:bi + seqs_per_batch]

                    # Forward & Clamp logits
                    b_h = s_hid[idx].unsqueeze(0)
                    logits, values_mo = model.forward_sequence(
                        s_feat[idx], s_maps[idx], b_h, s_don[idx])
                    logits = torch.clamp(logits, -LOGIT_CLAMP, LOGIT_CLAMP)

                    # Flatten them all
                    b_acts = s_acts[idx].reshape(-1)
                    b_ret = s_ret[idx].reshape(-1, n_obj)
                    b_wgt = s_wgt[idx].reshape(-1)

                    # We calculate loss here (cross entropy, you see)
                    ce = nn.functional.cross_entropy(
                        logits, b_acts, reduction="none")
                    pl = (ce * b_wgt).sum() / (b_wgt.sum() + 1e-8)

                    # ... and entropy & log probs
                    probs = torch.softmax(logits, dim=-1)
                    lp = torch.log(probs + 1e-10)
                    entropy = -(probs * lp).sum(dim=-1).mean()

                    # Main squared error
                    vl_per_obj = []
                    for o in range(n_obj):
                        vl_o = nn.MSELoss()(values_mo[:, o], b_ret[:, o])
                        vl_per_obj.append(vl_o)

                    vl_total = sum(vl_per_obj)
                    loss = pl + VALUE_COEFF * vl_total

                    # typical update here, applies almost to all ML fields.
                    optimizer.zero_grad()
                    loss.backward()
                    nn.utils.clip_grad_norm_(model.parameters(), MAX_GRAD_NORM)
                    optimizer.step()

                    tot_pl += pl.item()
                    for o in range(n_obj):
                        tot_vl[o] += vl_per_obj[o].item()
                    tot_ent += entropy.item()
                    tot_kl += 0.0
                    tot_loss += loss.item()
                    n_upd += 1
        else:
            # Since the code are same, I might combine later? Or keep them but anyway no not necessary comments.
            norm_advantages = np.zeros_like(all_advantages)
            for o in range(n_obj):
                adv_o = all_advantages[:, o]
                std = adv_o.std()
                if std > 1e-5:
                    norm_advantages[:, o] = (adv_o - adv_o.mean()) / (std + 1e-8)
                else:
                    norm_advantages[:, o] = np.zeros_like(adv_o)

            lam_val = torch.exp(log_lambda).item()
            combined_advantages = (
                    norm_advantages[:, 1]
                    + norm_advantages[:, 2]
                    + lam_val * norm_advantages[:, 0]
            )
            combined_advantages = (combined_advantages - combined_advantages.mean()) / \
                                  (combined_advantages.std() + 1e-8)

            # In a PDO algorithm for solving (\ref{cpolopt}), dual variables would be updated according to
            # %
            # \begin{equation}
            # \nu_{k+1} = \left(\nu_k + \alpha_k \left(J_C (\pi_k) - d\right)\right)_+, \label{pdodual}
            # \end{equation}
            # %
            death_rate = (rewards_buf[:, 0] < -0.25).mean()
            constraint_violation = death_rate - MAX_DEATH_RATE
            with torch.no_grad():
                log_lambda += DUAL_LR * constraint_violation
                log_lambda.clamp_(np.log(DUAL_MIN), np.log(DUAL_MAX))

            num_seqs = H // SEQ_LEN
            H_trunc = num_seqs * SEQ_LEN
            s_feat = torch.from_numpy(features_buf[:H_trunc].reshape(
                num_seqs, SEQ_LEN, FEATURE_DIM).copy()).to(DEVICE)
            s_maps = torch.from_numpy(maps_buf[:H_trunc].reshape(
                num_seqs, SEQ_LEN, MAP_CHANNELS, grid_h, grid_w).copy()).to(DEVICE)
            s_acts = torch.from_numpy(actions_buf[:H_trunc].reshape(
                num_seqs, SEQ_LEN).copy()).to(DEVICE)
            s_olp = torch.from_numpy(log_probs_buf[:H_trunc].reshape(
                num_seqs, SEQ_LEN).copy().astype(np.float32)).to(DEVICE)
            s_adv = torch.from_numpy(combined_advantages[:H_trunc].reshape(
                num_seqs, SEQ_LEN).astype(np.float32)).to(DEVICE)
            s_ret = torch.from_numpy(all_returns[:H_trunc].reshape(
                num_seqs, SEQ_LEN, n_obj).astype(np.float32)).to(DEVICE)
            s_don = torch.from_numpy(dones_buf[:H_trunc].reshape(
                num_seqs, SEQ_LEN).copy().astype(np.float32)).to(DEVICE)
            s_hid = torch.from_numpy(hidden_buf[:H_trunc:SEQ_LEN].copy()).to(DEVICE)
            # No weights, notice that.

            # Do a curriculum for entropy coeff
            entropy_coeff = ENTROPY_COEFF_START + \
                            min(1.0, update_step / ENTROPY_ANNEAL_UPDATES) * \
                            (ENTROPY_COEFF_END - ENTROPY_COEFF_START)

            seqs_per_batch = min(SEQS_PER_BATCH, num_seqs)
            tot_pl, tot_ent, tot_kl, tot_loss, n_upd = 0, 0, 0, 0, 0
            tot_vl = np.zeros(n_obj, dtype=np.float64)

            for _ in range(PPO_EPOCHS):
                perm = torch.randperm(num_seqs)
                for bi in range(0, num_seqs, seqs_per_batch):
                    idx = perm[bi:bi + seqs_per_batch]

                    b_h = s_hid[idx].unsqueeze(0)
                    logits, values_mo = model.forward_sequence(
                        s_feat[idx], s_maps[idx], b_h, s_don[idx])
                    logits = torch.clamp(logits, -LOGIT_CLAMP, LOGIT_CLAMP)

                    b_acts = s_acts[idx].reshape(-1)
                    b_olp = s_olp[idx].reshape(-1)
                    b_adv = s_adv[idx].reshape(-1)
                    b_ret = s_ret[idx].reshape(-1, n_obj)

                    probs = torch.softmax(logits, dim=-1)
                    lp = torch.log(probs + 1e-10)
                    new_lp = lp.gather(1, b_acts.unsqueeze(1)).squeeze(1)

                    # This time not cross entropy because we are not doing SL. Standard PPO update
                    entropy = -(probs * lp).sum(dim=-1).mean()
                    ratio = torch.exp(new_lp - b_olp)
                    s1 = ratio * b_adv
                    # Clamping of PPO.
                    s2 = torch.clamp(ratio, 1.0 - CLIP_EPSILON,
                                     1.0 + CLIP_EPSILON) * b_adv
                    pl = -torch.min(s1, s2).mean()

                    vl_per_obj = []
                    for o in range(n_obj):
                        vl_o = nn.MSELoss()(values_mo[:, o], b_ret[:, o])
                        vl_per_obj.append(vl_o)

                    vl_total = sum(vl_per_obj)
                    loss = pl + VALUE_COEFF * vl_total - entropy_coeff * entropy

                    optimizer.zero_grad()
                    loss.backward()
                    nn.utils.clip_grad_norm_(model.parameters(), MAX_GRAD_NORM)
                    optimizer.step()

                    tot_pl += pl.item()
                    for o in range(n_obj):
                        tot_vl[o] += vl_per_obj[o].item()
                    tot_ent += entropy.item()
                    tot_kl += (b_olp - new_lp).mean().item()
                    tot_loss += loss.item()
                    n_upd += 1

        # Output for logs
        update_step += 1
        policy_version = update_step
        ms = (time.time() - t_ppo) * 1000
        lam_now = torch.exp(log_lambda).item()
        logging.info(f"moppo: #{update_step} {ms:.0f}ms  "
                     f"pi={tot_pl / n_upd:.4f} "
                     f"v_s={tot_vl[0] / n_upd:.4f} v_c={tot_vl[1] / n_upd:.4f} "
                     f"v_r={tot_vl[2] / n_upd:.4f} "
                     f"H={tot_ent / n_upd:.4f} ec={entropy_coeff:.5f} "
                     f"lambda={lam_now:.3f} dr={death_rate:.5f}")
        log_update(episode, update_step, tot_pl / n_upd,
                   tot_vl[0] / n_upd, tot_vl[1] / n_upd, tot_vl[2] / n_upd,
                   tot_ent / n_upd, tot_kl / n_upd, tot_loss / n_upd,
                   lam_now, death_rate, cfg, rs, rg)

        # CC advancement, rollout s. if success then success see if fail... one success is enough to adcance.
        event = "success" if rs > 0 else ("fail" if rg > 0 else None)
        if event is not None:
            old_cfg = cfg
            cfg = advance_cfg(cfg, event)
            if cfg != old_cfg:
                config_version += 1
                write_cfg(cfg)
                logging.info(
                    f"host: Curriculum {event}: {cfg_debug(old_cfg)} -> {cfg_debug(cfg)}"
                )
                for pipe in host_pipes:
                    pipe.send({"type": "config", "version": config_version})

        # All workers, update your weight! Else your guys cannot keep the trend up and will become useless!
        new_weights = serialize_weights()
        for pipe in host_pipes:
            pipe.send({"type": "weights", "data": new_weights, "version": policy_version})
        if not OFF_POLICY:
            wait_for_workers("ready", policy_version)
            for pipe in host_pipes:
                pipe.send({"type": "resume", "version": policy_version})

        # Save checkpoint
        torch_save({
            "model_state": model.state_dict(),
            "optimizer_state": optimizer.state_dict(),
            "episode": episode,
            "playperf": playperf,
            "update_step": update_step,
            "log_lambda": log_lambda.item(),
            "cfg": cfg_to_dict(cfg),
        }, ckpt_path)
        save_curriculum_state(cfg, episode, update_step)
