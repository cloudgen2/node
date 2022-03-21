#!/bin/sh
find riodefi -name "Cargo.toml" -exec sh -xec "sed 's,polkadot-v0\.9\.9,polkadot-v0.9.10,g' < {} > {}.tmp; mv {}.tmp {}" \;
