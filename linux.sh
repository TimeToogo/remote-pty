#/bin/bash

docker build -t build-ubuntu $(dirname $0)
docker run --privileged --rm -it \
    -v /var/run/docker.sock:/var/run/docker.sock \
    -p 8888:8888 \
    -v$(realpath $(dirname $0)):/app \
    -w/app \
    -eCARGO_TARGET_DIR=/tmp \
    build-ubuntu $@ 
