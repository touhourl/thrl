# Contributing

Thank you for considering a contribution to this project! Contributions of all kinds are welcome, including bug reports, feature requests, game suggestions, documentation improvements, tests, and code changes.
Also, if you find some grammatical issue or speling issues in any documentation (excluding code files, src/ and core/), those are also considered as contributions.
An "It works! It starts successfully on (your machine)" issue is also useful.
## Before You Start

Please:

1. Search the existing issues and pull requests to avoid duplicates.
2. Review the project's README and documentation (docs/).
3. For significant changes, open an issue before starting implementation so the approach can be discussed. Please discuss inside the issue so we can be 100% certain that it is necessary.

## I found a point where can be optimized!

This is crucial. I am not so interested in Python as you see, because I don't have good hardware for using it (not theory). So, please **do** tell me...

## I found an obvious bug!

Oh, please tell me! It is best to include:

* Steps to reproduce the problem
* Expected behavior
* Actual behavior
* GPU (cuda, xpu, etc.), Linux version, Torch version, issue in the upstream repository, Python version, etc.
* The project version or commit
* Relevant logs, screenshots, error messages, or anything that can be useful

Remove any personal and copyrighted information and follow copyright law and other applicable laws.

## I want to add more games!

If you really want to add a game, please describe:

1. Difficulty of including it
2. Copyright status & risks
3. If applicable, source code or example DLL / .so injection
   
See docs/game.tex for more information.

## I have a question!

Open an issue. Make sure you search before opening it.

## I found a solution to a bug!

Still, it is very useful for me. Please tell me the solution and, if applicable, all related information.

## I designed a new algorithm!

Congratulations! But you should write a $\LaTeX$ document about it. Hardware constraints, libraries used, memory limits, map/schema changes, convergence conditions, etc...
Also, you can use this repo, cite this repo, and develop your new paper. I hope you can give me your better implementation of the algorithm, thank you.

## Writing code

Define `author` as any person or thing that **did any part of a contribution**.

Define `contributor` as the person who **submits** the PR or an issue.

Contributors and authors must understand their code deeply. A contributor must keep the work under GPL-v3 terms and be responsible for anything that the author did.
You can be an author and a contributor at the same time. A contributor must clearly state the author's name and sources.
A PR can only be written by the contributor. If a contributor submits code without understanding what the author did, the code can only be submitted by the author.
A contributor cannot do any style of matrix multiplication while contributing, except for testing the code. We strictly don't allow any matrix multiplication in any PR's documentation.

One sentence: try to not use generative tools (not shell or perl or python tho) while contributing, and all documentation must be written by yourself. 
I do not want to mention that [10, 12], [6].MD, [6].MD, [6].MD, [5, 8, 5] in this repo because using them is not good for the Earth, and everyone just becomes a fake person. I don't like that. 
Because this project has also done a lot of damage to Earth and environment, I regret, but it is like that trend, everyone has nothing to stop it.

Jokes can be made in code, and this is actually preferred. But:

1. Documentation in docs/ must be serious
2. Good jokes only, without harassment or targeted attacks.
3. Non-ASCII characters are not allowed usually; this is not allowed: e.g. 権限符「This incident will be reported」. Use $\LaTeX$: $$a_t = \arg \max_a \left[\log\pi(a \mid s_t) + g_a\right], \quad g_a \sim -\log(-\log U[0,1])$$
4. For Python, you should write a docstring for every function.
Clean up comments and code you don't need anymore, if unused comments or code in the repo, please see if they are stable and then consider removing/changing them, don't learn from me...

## License

By submitting a contribution, you agree that your code may be distributed under the GPL-v3-or-later license, and your documentation may be distributed under the CC BY-SA 4.0 license.
