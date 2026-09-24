"""Entry point."""
#os.environ["SYCL_CACHE_PERSISTENT"] = "1"
#os.environ["SYCL_DEVICE_FILTER"] = "level_zero:gpu:0"
#os.environ["SYCL_CACHE_DIR"] = os.path.expanduser("~~~~/~~~.cache/sycl") // how stupid I was forgetting...
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
    train()
