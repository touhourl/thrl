"""Entry point."""
# We do not need the hacks of sycl anymore.
import logging
import sys
from core.algorithms import train

logging.basicConfig(
    level=logging.INFO,
    format="%(asctime)s %(levelname)s %(message)s",
    stream=sys.stdout,
    force=True,
)

if __name__ == "__main__":
    train(eval_mode=sys.argv[1:] == ["eval"])
