#!/bin/sh
set -ex

cargo build --release
chmod -R a+r db
rsync target/release/ajdb-web ajdb.hu:/opt/ajdb/ajdb-web-new
rsync -a src/web/static/ ajdb.hu:/opt/ajdb/src/web/static/
rsync -a db/ ajdb.hu:/opt/ajdb/db/
ssh ajdb.hu -A "
    cd /opt/ajdb;
    sudo systemctl stop ajdb-web &&
    mv ajdb-web-new ajdb-web &&
    sudo systemctl start ajdb-web &&
    echo 'Successful'
"

