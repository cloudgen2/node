FROM ubuntu:20.04

ENV USER=ubuntu
ENV WORK_DIR=/home/$USER

# install dependencies
RUN apt update
RUN apt install xz-utils curl sudo -y

# create user
RUN useradd -rm -d $WORK_DIR -s /bin/bash -g root -G sudo -u 1001 $USER
RUN install -d -m755 -o $(id -u) -g $(id -g) /nix
RUN chown -R $USER /nix

# for local development purposes
# - set root password to password
# - to switch to root run `su - root`
#RUN echo 'root:password' | chpasswd

# copy sources
WORKDIR $WORK_DIR
COPY . ./src
RUN chown -R $USER $WORK_DIR/src

# switch to created user
USER $USER

# install nix
# fixing nix version due to bug with nix 2.5
# - https://github.com/NixOS/nix/issues/5776
#RUN curl -L https://nixos.org/nix/install | sh
RUN curl -L https://releases.nixos.org/nix/nix-2.4/install | sh

# configure build environment
# see: $HOME/.nix-profile/etc/profile.d/nix.sh
ENV PATH="$WORK_DIR/.nix-profile/bin:$PATH"
ENV NIX_SSL_CERT_FILE=/etc/ssl/certs/ca-certificates.crt
ENV NIX_PROFILES="/nix/var/nix/profiles/default $HOME/.nix-profile"

ENV RUST_BACKTRACE=1

# docker build . --build-arg build=skip
ARG build
RUN if [ "$build" = "skip" ] ; \
    then \
        echo 'Build skipped!' ; \
    else \
        sh -c '~/src/docker/scripts/build.sh' ; \
    fi

# build node
#RUN cd ./src \
#    && nix-shell -I nixpkgs=channel:nixos-21.05 third-party/nix/shell.nix --run "cargo build --release"

# standard ports
#EXPOSE 30333 30334 9933 9944

# ports based on Jania documentation
# HTTP
EXPOSE 9933
# WS
EXPOSE 9944
# Prometheus exporter
EXPOSE 9615
# p2p flow
EXPOSE 30333

# RPC
EXPOSE 80

#CMD ./docker/scripts/run.sh

ENTRYPOINT ["/home/ubuntu/src/target/release/astar-collator"]
CMD ["--chain", "dev2", "--tmp", "--rpc-external", "--ws-external", "--rpc-port", "80", "--rpc-cors", "all"]