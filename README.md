# thrl - A Touhou Reinforcement Learning Framework
Cite this repository as following:
```
@article{Liu_A_High-Fidelity_Reinforcement_2026,
author = {Liu, T.},
doi = {10.5281/zenodo.21788472},
title = {{A High-Fidelity Reinforcement Learning Environment and Baseline for Multi-Objective Bullet Hell Games}},
year = {2026}
}
```
# Requirements
Currently, only GNU/Linux is supported. I do not have the enough time to do for Windows. You can try WSL2, but I won't promise that works.

You must have a GPU. Supported GPU in this repo are [XPU](https://pytorch.org/get-started/additional-platforms/) and [CUDA](https://pytorch.org/).

Use `make switch` to switch GPU before running any command.

GPU must have >=6 GB VRAM. Integrated (shared memory) can also work.

One 2,048-step map rollout is about 1.62 GiB. But even though, system RAM can be very high due to processes, spawn and pytorch clones.
It has about 22GB usage on an Intel XPU Shared Memory (default configuration). If time is not a issue, just use default config.
I estimate that if we are using CUDA, the needed GPU VRAM will be about 3-6 GB and system memory will be about 12-15 GB.

The safest is one worker. It can even run on 4 (VRAM) + 8 (RAM) devices.

Test how long the MOPPO update needs. You could add more workers if it is quick, so it can keep up more. else, reduce the amount of workers.

Intel XPU has a bug: it rebuilds sycl every time if you don't cache. So, you can enable the things in main.py and let it cache.

## Get started
To get it running, you need a valid game copy of any games you want to run with.

For PC-98 era games: 
1. Clone dosbox-x and build it from source:
    ```shell
    sudo apt install curl automake gcc g++ make libncurses-dev nasm libsdl-net1.2-dev libsdl2-net-dev libpcap-dev libslirp-dev fluidsynth libfluidsynth-dev libavformat-dev libavcodec-dev libavcodec-extra libswscale-dev libfreetype-dev libxkbfile-dev libxrandr-dev
    git clone https://github.com/joncampbell123/dosbox-x.git
    cd dosbox-x
    ./build
    sudo make install
    ```
2. Install rust toolchains:
    ```shell
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
    ```
3. Install `uv` and `maturin`:
    ```shell
    curl -LsSf https://astral.sh/uv/install.sh | sh
    uv tool install maturin
    ```
4. Patch your game executables. See repo th05patch. Place them at ./export/. 
5.  ```shell
    maturin develop
    uv sync
    sudo $(eval echo ~$(whoami))/.local/bin/uv run main.py
    ```
    If using xvfb:
    ```shell
    sudo su
    /path/to/uv run main.py
    ```
Fix permission issues:
```shell
sudo chmod -R 777 ./target .venv/
```
Build documentation:
```shell
make docs
```
# Donating

But if you really has money, donate to 0x0Ec67fa7d7Fbe849D481F32ee24CCecE901B3F55 at Ethereum with ETH/USDT/USDC or 
Arbitrum One with ETH.
Donation will be processed via cryptocurrencies instead of any other ways (I am too lazy to set it up).

# Why not crowdfunding?

Yes, I want credit and money. But, this project is not as same as ReC98. It needs public contributions, advanced knowledge 
in reinforcement learning and scientific researches, and I believe you can do better than me.

# How can it converge?

I will put it temporary in readme. During my first experiment, a TH04 experiment (source code is already deleted) which I used only
Top-K things and standard PPO approach, had a good result. The agent went further to stage 2 (idx 1) and fought with boss. 
The training lasted about 20 days and had to stop because of emergency shutdown. It went really slow (~ 3 min per episode 
because of original executable, entirely CPU, and /dev/input instead of memory writes), so it was not efficient.
With the current implementation status, I bet someone with a good hardware can run it. 

Entropy goes down, but it cannot fully beat stage 3 (idx 2) in game with 3 lives and 3 bombs. But in my opinion, it is only a time issue.

In someone's RL debugging suggestions (if I am correct it is a video), I can be sure:

1. Reward algorithm is not a problem.
2. This isn't a simulator.

I cannot be sure:

1. Algorithm does not have any problems, because this project is not peer-reviewed
2. I have enough power to train it.

So what is XPU? Well, I have only an "Intel Corporation Meteor Lake-P \[Intel Arc Graphics\]", that is integrated. So, you see well the speed.
What is integrated? It shares memory with sys memory and is very slow. As a small model (e.g. llama-2-7b.Q4_K_M.gguf), it runs at 2 tok/s.
A MOPPO update (2048 steps) in this repo, it spends 18-36 seconds. 
So you see why I am complaining it all the time.

# Roadmap

I will also develop it further, adding more games. 3 Games are already in planing and one of them has been written the draft. 
TH04 is one of it and will be delivered as soon as I understood the TH04's CustomEntity. I love all musics in PC-98 era.
But first, lemme take first a month break and recover from depressions and overloaded work,
spending time with RL (real life).

Original README:
---
We Reinforcement Learning.
We Reverse Engineering.
We Rust.

The RL agent for Bullet Hell Games.

Written by a person in 6 months, only one person. Doing it every day.

Thanks ReC98 for detailed reverse engineered code and blogs, StableBaseline3 for some implementations in Python, PyTorch for Intel/CUDA acceleration,
GNU Project for license and OS, Professor Donald E. Knuth for TeX, arxiv.org for fantastic papers (though I hate it also), Institute of Electrical and Electronics Engineers for some papers.

Without them, such a project is never possible.
