#!/bin/sh
set -ex

cargo build --release
rsync target/release/ajdb-web ajdb.hu:/opt/ajdb/ajdb-web-new
rsync -a src/web/static/ ajdb.hu:/opt/ajdb/src/web/static/
ssh ajdb.hu -A "
    cd /opt/ajdb;
    sudo systemctl stop ajdb-web &&
    mv ajdb-web-new ajdb-web &&
    sudo systemctl start ajdb-web &&
    echo 'Successful'
"

