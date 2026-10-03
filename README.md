# thrl - A Touhou Reinforcement Learning Framework
Cite the **paper** as following:
```
@article{liu2026highfidelity,
    author = {Liu, T.},
    doi = {10.5281/zenodo.21788472},
    title = {{A High-Fidelity Reinforcement Learning Environment and Baseline for Multi-Objective Bullet Hell Games}},
    year = {2026}
}
```
# Links
Taisei Headless Simulation: [https://github.com/touhourl/taisei-sim](https://github.com/touhourl/taisei-sim)

th98patch (patch for Mystic Square and Lotus Land Story): [https://codeberg.org/thrl/th98patch](https://codeberg.org/thrl/th98patch)

thrl: [https://codeberg.org/thrl/thrl](https://codeberg.org/thrl/thrl)

# Requirements

Only GNU/Linux is supported. WSL2 is okay.

You must have a GPU. Supported GPU in this repo are [XPU](https://pytorch.org/get-started/additional-platforms/) and [CUDA](https://pytorch.org/).

Use `make switch` to switch GPU before running any command.

GPU must have >=4 GB VRAM. Integrated (shared memory) can also work.

One 2,048-step map rollout is about 1.62 GiB. But even though, system RAM can be very high due to processes, spawn and pytorch clones.
It has about 11GB usage on an Intel XPU Shared Memory (default configuration, 4 workers). If time is not a issue, just use default config.
I estimate that if we are using CUDA, the needed GPU VRAM will be about 3-6 GB and system memory will be about 7-10 GB.

The safest is one worker. It can even run on 4 (VRAM) + 4 (RAM) (or 8 RAM if XPU) devices.
Note: the RAM noticed are all needed RAM, not actually system RAM. Suppose you just use tty1 mode.
Then overhead of sys is around 1 GB? And then run it easily with 9 GB (tho there isn't one).

Test how long the MOPPO update needs. You could add more workers if it is quick, so it can keep up more. Else, reduce the amount of workers.

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
    sudo setcap -r "$(type -P dosbox-x)" # This doesn't affect game only network
    ```
    Note: Every time you do `git pull && ./build && sudo make install`, you need to do
    `sudo setcap -r "$(type -P dosbox-x)"` also, so running it does not require root, except HM mode.
2. Install rust toolchains:
    ```shell
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
    ```
3. Install `uv` and `maturin`:
    ```shell
    curl -LsSf https://astral.sh/uv/install.sh | sh
    uv tool install maturin
    ```
4. Patch your game executables. See repo th98patch. Place them at ./export/. 
5. Use `make defconfig` to generate default configurations.
   Configure runtime settings in `rrr.toml` and `curriculum.json`. You may use default config via `make defconfig`.
   Use `cp cfg/dosbox-x/test.conf export/default.conf` for testing, debugging, evulating and human mode.
   Use `cp cfg/dosbox-x/headless.conf export/default.conf` for normal training.
6.  ```shell
    maturin develop
    uv sync
    uv run main.py
    ```

Build documentation:
```shell
make docs
```

# Donating

Donate to 0x0Ec67fa7d7Fbe849D481F32ee24CCecE901B3F55 at Ethereum with ETH/USDT/USDC or 
Arbitrum One with ETH.
Donation will be processed via cryptocurrencies instead of any other ways. 
The donated costs will be used with experiment costs and cloud compute costs.

# Experiment

I will put it temporary in readme. I do not have enough compute power, so it is unknown.

In someone's RL debugging suggestions (if I am correct it is a video), I can be sure:

1. Reward algorithm is not a problem.
2. This isn't a simulator.
3. I do not enough money to train it.

I cannot be sure, algorithm does not have any problems, because this project is not peer-reviewed.

It could not run at any big tech cloud computes at free tier, as I tested. 

# Special thanks

ReC98 for detailed reverse engineered code and blogs, StableBaseline3 for some implementations in Python, PyTorch for Intel/CUDA acceleration,
GNU Project for license and OS, Professor Donald E. Knuth for TeX, arxiv.org for fantastic papers (though I hate it also), Institute of Electrical and Electronics Engineers for some papers.
