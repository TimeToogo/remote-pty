#/bin/bash

docker build -t build-ubuntu .
docker run --rm -it -v$PWD:/app -w/app build-ubuntu $@ 
