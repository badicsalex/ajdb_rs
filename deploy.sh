#!/bin/sh
set -x

cargo build --release
scp target/release/ajdb-web ajdb.hu:/opt/ajdb/ajdb-web-new
rsync -a src/web/static /opt/ajdb/src/web/static
ssh ajdb.hu -A "
    cd /opt/ajdb;
    sudo systemctl stop ajdb-web &&
    mv ajdb-web-new ajdb-web &&
    sudo systemctl start ajdb-web &&
    echo 'Successful'
"

