#/bin/bash

docker build -t build-ubuntu .
docker run --privileged --rm -it \
    -v /var/run/docker.sock:/var/run/docker.sock \
    -v$PWD:/app \
    -w/app \
    build-ubuntu $@ 
