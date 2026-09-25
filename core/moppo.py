"""Most important trainer.
"""

"""
    MOPPO trainer of thrl.
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

import json
import logging
import math
import os
import threading
import time

import rrr
import torch
import torch.nn as nn
import torch.optim as optim

from .algorithms import compute_gae, entropy_coeff as get_entropy_coeff
from .model import MOActorCritic


def train() -> None:
    """Main trainer (dummy docstring here)

    :raises ___ don't know, any errors
    :return None
    """
    rrr.init_logging()
    runtime_config = json.loads(rrr.runtime_config_json())
    runtime_cfg = runtime_config["runtime"]
    paths_cfg = runtime_config["paths"]
    training_cfg = runtime_config["training"]
    worker_cfg = runtime_config["worker"]
    model_cfg = runtime_config["model"]
    gru_cfg = runtime_config["gru"]
    env_spec = json.loads(rrr.environment_spec_json())
    lagrangian_cfg = runtime_config["lagrangian"]

    if runtime_cfg["algorithm"] != "moppo":
        raise RuntimeError(f"Unsupported trainer: {runtime_cfg['algorithm']}")

    RL_BY_HUMAN = runtime_cfg["rl_by_human"]
    RESET_OPTIMIZER_ON_RESUME = runtime_cfg["reset_optimizer_on_resume"]
    RESET_LAMBDA_ON_RESUME = runtime_cfg["reset_lambda_on_resume"]
    MODEL_SAVE_DIR = paths_cfg["model_save_dir"]
    os.makedirs(MODEL_SAVE_DIR, exist_ok=True)
    LEARNING_RATE = training_cfg["learning_rate"]
    GAMMA = training_cfg["gamma"]
    LAMBDA_GAE = training_cfg["lambda_gae"]
    ENTROPY_COEFF_START = training_cfg["entropy_coeff_start"]
    ENTROPY_COEFF_END = training_cfg["entropy_coeff_end"]
    ENTROPY_ANNEAL_UPDATES = training_cfg["entropy_anneal_updates"]
    CLIP_EPSILON = training_cfg["clip_epsilon"]
    VALUE_COEFF = training_cfg["value_coeff"]
    PPO_EPOCHS = training_cfg["ppo_epochs"]
    HUMAN_BC_EPOCHS = training_cfg["human_bc_epochs"]
    HUMAN_ADV_WEIGHT_BETA = training_cfg["human_adv_weight_beta"]
    HUMAN_ADV_WEIGHT_MIN = training_cfg["human_adv_weight_min"]
    HUMAN_ADV_WEIGHT_MAX = training_cfg["human_adv_weight_max"]
    HORIZON = training_cfg["horizon"]
    LOGIT_CLAMP = training_cfg["logit_clamp"]
    MAX_GRAD_NORM = training_cfg["max_grad_norm"]
    FEATURE_PROJECT_DIM = model_cfg["feature_project_dim"]
    CNN_EMBED_DIM = model_cfg["cnn_embed_dim"]
    CNN_POOL_OUT = tuple(model_cfg["cnn_pool_out"])
    CNN_HIDDEN_CHANNELS = model_cfg["cnn_hidden_channels"]
    HIDDEN_DIM = model_cfg["hidden_dim"]
    GRU_HIDDEN_SIZE = model_cfg["gru_hidden_size"]
    NUM_OBJECTIVES = env_spec["rewards"]
    ACTION_DIM = env_spec["actions"]
    SEQ_LEN = gru_cfg["seq_len"]
    SEQS_PER_BATCH = gru_cfg["seqs_per_batch"]
    CHUNK_SIZE = worker_cfg["chunk_size"]
    if SEQ_LEN <= 0 or CHUNK_SIZE % SEQ_LEN != 0:
        raise RuntimeError(
            f"MOPPO requires chunk_size ({CHUNK_SIZE}) to be divisible by "
            f"`seq_len` ({SEQ_LEN})"
        )
    FEATURE_DIM = env_spec["observations"][0][0]
    MAP_CHANNELS, GRID_H, GRID_W = env_spec["observations"][1]
    MAX_DEATH_RATE = lagrangian_cfg["max_death_rate"]
    DUAL_LR = lagrangian_cfg["dual_lr"]
    DUAL_MAX = lagrangian_cfg["dual_max"]
    DUAL_MIN = lagrangian_cfg["dual_min"]
    INITIAL_LAMBDA = lagrangian_cfg["initial_lambda"]

    # my favourite f***ing xpu. I love it so much.
    DEVICE = torch.device(
        "xpu"
        if hasattr(torch, "xpu") and torch.xpu.is_available()
        else ("cuda" if torch.cuda.is_available() else "cpu")
    )
    logging.info(f"moppo: Using {DEVICE}")

    n_obj = NUM_OBJECTIVES
    grid_h, grid_w = GRID_H, GRID_W

    model = MOActorCritic(
        feature_dim=FEATURE_DIM,
        map_channels=MAP_CHANNELS,
        cnn_embed_dim=CNN_EMBED_DIM,
        hidden_dim=HIDDEN_DIM,
        action_dim=ACTION_DIM,
        feature_proj_dim=FEATURE_PROJECT_DIM,
        gru_hidden_size=GRU_HIDDEN_SIZE,
        num_objectives=NUM_OBJECTIVES,
        cnn_hidden_channels=CNN_HIDDEN_CHANNELS,
        cnn_pool_out=CNN_POOL_OUT,
    ).to(DEVICE)
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
    log_lambda = torch.tensor(math.log(INITIAL_LAMBDA), dtype=torch.float32,
                              device=DEVICE, requires_grad=False)

    start_ep, update_step = 0, 0
    cfg_json = None
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
        update_step = ckpt.get("update_step", 0)  # this is accurate anyway

        # reset lambda cuz old checkpoint had lambda pollution (when testing, also normally no need, see docs)
        if RESET_LAMBDA_ON_RESUME:
            lambda_init = min(max(float(INITIAL_LAMBDA), DUAL_MIN), DUAL_MAX)
            log_lambda = torch.tensor(
                math.log(lambda_init),
                dtype=torch.float32,
                device=DEVICE
            )
            logging.warning(f"Resetting lambda to {lambda_init:.4f}")
        else:
            lambda_loaded = ckpt.get("log_lambda", math.log(INITIAL_LAMBDA))
            lambda_loaded = min(max(math.exp(float(lambda_loaded)), DUAL_MIN), DUAL_MAX)
            log_lambda = torch.tensor(
                math.log(lambda_loaded),
                dtype=torch.float32,
                device=DEVICE
            )

        cfg_value = ckpt.get("cfg")
        if isinstance(cfg_value, str):
            cfg_json = cfg_value
        elif cfg_value:
            cfg_json = json.dumps(cfg_value, sort_keys=True)

        logging.info(f"Resumed ep={start_ep}, "
                     f"update={update_step}, lambda={torch.exp(log_lambda).item():.4f}"
                     )

    except Exception as e:
        logging.warning(f"Checkpoint load failed: {e}. Starting fresh.")

    collector = rrr.Collector(
        start_ep,
        update_step,
        cfg_json,
        state_size=GRU_HIDDEN_SIZE,
        aux_size=1 + NUM_OBJECTIVES,
    )
    update_step = collector.update_step()
    logging.info(f"ep={collector.episode()}, device={DEVICE}, "
                 f"updates={update_step}, lambda={torch.exp(log_lambda).item():.4f}")
    logging.info(f"Curriculum cfg: {collector.cfg_debug()}")

    inference_model = MOActorCritic(
        feature_dim=FEATURE_DIM,
        map_channels=MAP_CHANNELS,
        cnn_embed_dim=CNN_EMBED_DIM,
        hidden_dim=HIDDEN_DIM,
        action_dim=ACTION_DIM,
        feature_proj_dim=FEATURE_PROJECT_DIM,
        gru_hidden_size=GRU_HIDDEN_SIZE,
        num_objectives=NUM_OBJECTIVES,
        cnn_hidden_channels=CNN_HIDDEN_CHANNELS,
        cnn_pool_out=CNN_POOL_OUT,
    ).cpu()
    inference_model.load_state_dict(model.state_dict())
    inference_model.eval()
    inference_lock = threading.Lock()
    inference_errors = []

    def inference_loop():
        try:
            while True:
                batch = collector.inference_batch()
                if batch is None:
                    time.sleep(0.001)
                    continue
                forced_actions = batch["forced_actions"]
                batch_size = len(forced_actions)
                observations = batch["observations"]
                features = torch.frombuffer(
                    observations[0], dtype=torch.float32).reshape(batch_size, FEATURE_DIM)
                maps = torch.frombuffer(
                    observations[1], dtype=torch.float32).reshape(
                        batch_size, MAP_CHANNELS, grid_h, grid_w)
                hidden = torch.frombuffer(
                    batch["state"], dtype=torch.float32).reshape(
                        1, batch_size, GRU_HIDDEN_SIZE)

                # === POLICY INFERENCE === #
                with inference_lock, torch.no_grad():
                    logits, values_mo, new_hidden = inference_model.forward_step(features, maps, hidden)
                    logits = torch.clamp(logits, -LOGIT_CLAMP, LOGIT_CLAMP)
                    probs = torch.softmax(logits, dim=-1)  # Softmax policy, one of the Policy Gradient Methods

                    # Sample random & Grumbel, but we don't want log 0.
                    u = torch.rand_like(logits)
                    gumbel = -torch.log(-torch.log(u + 1e-10) + 1e-10)
                    # Gumbel-max trick, reference. Got em directly, so no additional proofs.
                    actions = (logits + gumbel).argmax(dim=-1)
                    # @misc{huijben2022reviewgumbelmaxtrickextensions,
                    # title={A Review of the Gumbel-max Trick and its Extensions for Discrete Stochasticity in Machine Learning},
                    # author={Iris A. M. Huijben and Wouter Kool and Max B. Paulus and Ruud J. G. van Sloun},
                    # year={2022},
                    # eprint={2110.01515},
                    # archivePrefix={arXiv},
                    # primaryClass={cs.LG},
                    # url={https://arxiv.org/abs/2110.01515},
                    # }
                    for i, forced_action in enumerate(forced_actions):
                        if forced_action is not None:
                            # Read the actual action taken by me.
                            actions[i] = int(forced_action)
                    log_probs = torch.log(
                        probs.gather(1, actions.unsqueeze(1)).squeeze(1) + 1e-10
                    )
                    # = \log \pi_{\theta_{old}}(a_t | s_t, h_t)

                aux = torch.cat((log_probs.unsqueeze(1), values_mo), dim=1)
                collector.submit_inference(
                    actions.detach().cpu().tolist(),
                    aux.detach().cpu().tolist(),
                    new_hidden.detach().cpu()[0].tolist(),
                )
        except Exception as e:
            inference_errors.append(e)

    inference_thread = threading.Thread(target=inference_loop, daemon=True)
    inference_thread.start()

    logging.info(f"Preparation Complete.")

    features_buf = torch.empty((HORIZON, FEATURE_DIM), dtype=torch.float32)
    maps_buf = torch.empty((HORIZON, MAP_CHANNELS, grid_h, grid_w), dtype=torch.float32)
    actions_buf = torch.empty(HORIZON, dtype=torch.int64)
    rewards_buf = torch.empty((HORIZON, n_obj), dtype=torch.float32)
    dones_buf = torch.empty(HORIZON, dtype=torch.float32)
    log_probs_buf = torch.empty(HORIZON, dtype=torch.float32)
    values_buf = torch.empty((HORIZON, n_obj), dtype=torch.float32)
    hidden_buf = torch.empty((HORIZON, GRU_HIDDEN_SIZE), dtype=torch.float32)
    acc_rewards = []
    acc_dones = []
    acc_values = []
    acc_last_values = []
    acc_steps = 0
    rollout_successes = 0
    rollout_gameovers = 0


    while True:
        while acc_steps < HORIZON:
            rollout = collector.next_rollout()
            if rollout is None:
                if inference_errors:
                    raise RuntimeError("Inference worker failed") from inference_errors[0]
                time.sleep(0.001)
                continue

            rollout_successes += rollout["successes"]
            rollout_gameovers += rollout["gameovers"]

            ns = rollout["n_steps"]
            observations = rollout["observations"]
            features = torch.frombuffer(
                observations[0], dtype=torch.float32).reshape(ns, FEATURE_DIM)
            maps = torch.frombuffer(
                observations[1], dtype=torch.float32).reshape(
                    ns, MAP_CHANNELS, grid_h, grid_w)
            actions = torch.frombuffer(
                rollout["actions"], dtype=torch.int64).reshape(ns)
            rewards = torch.frombuffer(
                rollout["rewards"], dtype=torch.float32).reshape(ns, n_obj)
            dones = torch.frombuffer(
                rollout["dones"], dtype=torch.float32).reshape(ns)
            aux = torch.frombuffer(
                rollout["aux"], dtype=torch.float32).reshape(ns, 1 + n_obj)
            log_probs = aux[:, 0]
            values = aux[:, 1:]
            hidden = torch.frombuffer(
                rollout["state"], dtype=torch.float32).reshape(ns, GRU_HIDDEN_SIZE)
            start = min(acc_steps, HORIZON)
            end = min(acc_steps + ns, HORIZON)
            if start < end:
                take = end - start
                features_buf[start:end].copy_(features[:take])
                maps_buf[start:end].copy_(maps[:take])
                actions_buf[start:end].copy_(actions[:take])
                rewards_buf[start:end].copy_(rewards[:take])
                dones_buf[start:end].copy_(dones[:take])
                log_probs_buf[start:end].copy_(log_probs[:take])
                values_buf[start:end].copy_(values[:take])
                hidden_buf[start:end].copy_(hidden[:take])
            acc_rewards.append(rewards)
            acc_dones.append(dones)
            acc_values.append(values)
            acc_last_values.append(torch.frombuffer(
                rollout["tail_aux"], dtype=torch.float32)[1:])
            acc_steps += ns

        collector.begin_update()

        # === (MO)PPO Update === #
        logging.info(f"moppo: Update {update_step} ({acc_steps} steps)...")

        # Compute GAE per chunk to avoid boud leaks
        chunk_advs = []
        chunk_rets = []
        for i in range(len(acc_rewards)):
            c_rew = acc_rewards[i]
            c_val = acc_values[i]
            c_done = acc_dones[i]
            c_lv = acc_last_values[i]

            c_adv = torch.zeros_like(c_rew)
            c_ret = torch.zeros_like(c_rew)
            for o in range(n_obj):
                adv_o, ret_o = compute_gae(
                    c_rew[:, o].tolist(),
                    c_val[:, o].tolist(),
                    c_done.tolist(),
                    float(c_lv[o]),
                    GAMMA,
                    LAMBDA_GAE,
                )
                c_adv[:, o] = torch.tensor(adv_o, dtype=torch.float32)
                c_ret[:, o] = torch.tensor(ret_o, dtype=torch.float32)
            chunk_advs.append(c_adv)
            chunk_rets.append(c_ret)

        # Throw all things that exceed Horizon away to keep it up.
        all_advantages = torch.cat(chunk_advs, dim=0)[:HORIZON]
        all_returns = torch.cat(chunk_rets, dim=0)[:HORIZON]

        H = len(features_buf)

        # Save rollout success/gameover stats before clearing
        rs = int(rollout_successes)
        rg = int(rollout_gameovers)

        acc_rewards.clear()
        acc_dones.clear()
        acc_values.clear()
        acc_last_values.clear()
        acc_steps = 0
        rollout_successes = 0
        rollout_gameovers = 0
        # Wasted
        _ = values_buf

        if RL_BY_HUMAN:
            logging.warning("Experimental feature. Ensure you use a DE and used dosbox-x/test.conf")
            # Human training
            # TODO: Experimental feature.
            # The PPO ratio cannot be applied here.
            # For each obj, do std deviation and avoid ner zero devide.
            norm_advantages = torch.zeros_like(all_advantages)
            for o in range(n_obj):
                adv_o = all_advantages[:, o]
                std = adv_o.std(unbiased=False)
                if std > 1e-5:
                    norm_advantages[:, o] = (adv_o - adv_o.mean()) / (std + 1e-8)
                else:
                    norm_advantages[:, o] = torch.zeros_like(adv_o)

            # Combine them but with lambda for survival
            lam_val = torch.exp(log_lambda).item()
            combined_advantages = (
                    norm_advantages[:, 1]
                    + norm_advantages[:, 2]
                    + lam_val * norm_advantages[:, 0]
            )

            # Normalizes the combined advantage. If not useful, we discard it.
            combined_std = combined_advantages.std(unbiased=False)
            if combined_std > 1e-5:
                combined_advantages = (combined_advantages - combined_advantages.mean()) / \
                                      (combined_std + 1e-8)
            else:
                combined_advantages = torch.zeros_like(combined_advantages)

            # Clipped like PPO so better things get more happened.
            human_weights = torch.exp(HUMAN_ADV_WEIGHT_BETA * combined_advantages)
            human_weights = torch.clamp(
                human_weights,
                HUMAN_ADV_WEIGHT_MIN,
                HUMAN_ADV_WEIGHT_MAX,
            )
            human_weights = human_weights / (human_weights.mean() + 1e-8)

            # If episode-level death, it will happen forever. because it will not pass mostly, so transitions.
            death_rate = (rewards_buf[:, 0] < -0.25).float().mean().item()

            # Keeps only data divisible by the seq len
            num_seqs = H // SEQ_LEN
            H_trunc = num_seqs * SEQ_LEN

            # Convert them all to tensors.
            s_feat = features_buf[:H_trunc].reshape(
                num_seqs, SEQ_LEN, FEATURE_DIM).to(DEVICE)
            s_maps = maps_buf[:H_trunc].reshape(
                num_seqs, SEQ_LEN, MAP_CHANNELS, grid_h, grid_w).to(DEVICE)
            s_acts = actions_buf[:H_trunc].reshape(
                num_seqs, SEQ_LEN).to(DEVICE)
            s_ret = all_returns[:H_trunc].reshape(
                num_seqs, SEQ_LEN, n_obj).to(DEVICE)
            s_don = dones_buf[:H_trunc].reshape(
                num_seqs, SEQ_LEN).to(DEVICE)
            s_hid = hidden_buf[:H_trunc:SEQ_LEN].to(DEVICE)
            s_wgt = human_weights[:H_trunc].reshape(
                num_seqs, SEQ_LEN).to(DEVICE)

            # I never know how to get a person's entropy, but for logging it is required right?
            # Update initialization
            entropy_coeff = 0.0
            seqs_per_batch = min(SEQS_PER_BATCH, num_seqs)
            tot_pl, tot_ent, tot_kl, tot_loss, n_upd = 0, 0, 0, 0, 0
            tot_vl = [0.0] * n_obj

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
            norm_advantages = torch.zeros_like(all_advantages)
            for o in range(n_obj):
                adv_o = all_advantages[:, o]
                std = adv_o.std(unbiased=False)
                if std > 1e-5:
                    norm_advantages[:, o] = (adv_o - adv_o.mean()) / (std + 1e-8)
                else:
                    norm_advantages[:, o] = torch.zeros_like(adv_o)

            lam_val = torch.exp(log_lambda).item()
            combined_advantages = (
                    norm_advantages[:, 1]
                    + norm_advantages[:, 2]
                    + lam_val * norm_advantages[:, 0]
            )
            combined_advantages = (combined_advantages - combined_advantages.mean()) / \
                                  (combined_advantages.std(unbiased=False) + 1e-8)

            # In a PDO algorithm for solving (\ref{cpolopt}), dual variables would be updated according to
            # %
            # \begin{equation}
            # \nu_{k+1} = \left(\nu_k + \alpha_k \left(J_C (\pi_k) - d\right)\right)_+, \label{pdodual}
            # \end{equation}
            # %
            death_rate = (rewards_buf[:, 0] < -0.25).float().mean().item()
            constraint_violation = death_rate - MAX_DEATH_RATE
            with torch.no_grad():
                log_lambda += DUAL_LR * constraint_violation
                log_lambda.clamp_(math.log(DUAL_MIN), math.log(DUAL_MAX))

            num_seqs = H // SEQ_LEN
            H_trunc = num_seqs * SEQ_LEN
            s_feat = features_buf[:H_trunc].reshape(
                num_seqs, SEQ_LEN, FEATURE_DIM).to(DEVICE)
            s_maps = maps_buf[:H_trunc].reshape(
                num_seqs, SEQ_LEN, MAP_CHANNELS, grid_h, grid_w).to(DEVICE)
            s_acts = actions_buf[:H_trunc].reshape(
                num_seqs, SEQ_LEN).to(DEVICE)
            s_olp = log_probs_buf[:H_trunc].reshape(
                num_seqs, SEQ_LEN).to(DEVICE)
            s_adv = combined_advantages[:H_trunc].reshape(
                num_seqs, SEQ_LEN).to(DEVICE)
            s_ret = all_returns[:H_trunc].reshape(
                num_seqs, SEQ_LEN, n_obj).to(DEVICE)
            s_don = dones_buf[:H_trunc].reshape(
                num_seqs, SEQ_LEN).to(DEVICE)
            s_hid = hidden_buf[:H_trunc:SEQ_LEN].to(DEVICE)
            # No weights, notice that.

            # Do a curriculum for entropy coeff
            entropy_coeff = get_entropy_coeff(
                update_step,
                ENTROPY_COEFF_START,
                ENTROPY_COEFF_END,
                ENTROPY_ANNEAL_UPDATES,
            )

            seqs_per_batch = min(SEQS_PER_BATCH, num_seqs)
            tot_pl, tot_ent, tot_kl, tot_loss, n_upd = 0, 0, 0, 0, 0
            tot_vl = [0.0] * n_obj

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
        lam_now = torch.exp(log_lambda).item()
        if inference_errors:
            raise RuntimeError("Inference worker failed") from inference_errors[0]
        collector.prepare_policy_update()
        with inference_lock:
            inference_model.load_state_dict(model.state_dict())
        update_record = {
            "episode": collector.episode(),
            "update_step": update_step,
            "policy_loss": tot_pl / n_upd,
            "value_loss": [value / n_upd for value in tot_vl],
            "entropy": tot_ent / n_upd,
            "entropy_coeff": entropy_coeff,
            "approx_kl": tot_kl / n_upd,
            "total_loss": tot_loss / n_upd,
            "lambda": lam_now,
            "cost_mean": death_rate,
            "rollout_successes": rs,
            "rollout_gameovers": rg,
            "cfg": json.loads(collector.cfg_json()),
        }
        collector.log_update(json.dumps(update_record, sort_keys=True))
        logging.info(
            f"moppo: #{update_step} pi={update_record['policy_loss']:.4f} "
            f"v={update_record['value_loss']} H={update_record['entropy']:.4f} "
            f"ec={entropy_coeff:.5f} lambda={lam_now:.3f} dr={death_rate:.5f}"
        )
        collector.finish_update(update_step, rs, rg)

        # Save checkpoint
        checkpoint = {
            "model_state": model.state_dict(),
            "optimizer_state": optimizer.state_dict(),
            "episode": collector.episode(),
            "update_step": update_step,
            "log_lambda": log_lambda.item(),
            "cfg": json.loads(collector.cfg_json()),
        }
        tmp = ckpt_path + ".tmp"
        torch.save(checkpoint, tmp)
        os.replace(tmp, ckpt_path)
