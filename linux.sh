#/bin/bash

docker build -t build-ubuntu $(dirname $0)
docker run --privileged --rm -it \
    -v /var/run/docker.sock:/var/run/docker.sock \
    -v$(realpath $(dirname $0)):/app \
    -w/app \
    build-ubuntu $@ 
