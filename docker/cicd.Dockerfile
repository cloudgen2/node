ARG FROM_IMAGE
FROM ${FROM_IMAGE}

WORKDIR /home/ubuntu
COPY . ./src

RUN sh -c '~/src/docker/scripts/build.sh'
