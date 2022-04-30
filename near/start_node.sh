#!/bin/bash

set -x

echo "starting web server"

python3 -m http.server --directory /tmp/_sandbox 3031 &

echo "starting near sandbox"
nearcore/target/release/near-sandbox --home /tmp/_sandbox run

