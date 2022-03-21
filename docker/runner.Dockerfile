ARG FROM_IMAGE
FROM ${FROM_IMAGE} as rio-build

FROM nixos/nix

WORKDIR /root/
COPY --from=rio-build /home/ubuntu/src/target/release/astar-collator .
RUN find .

# HTTP
EXPOSE 80
# WS
EXPOSE 9944
# Prometheus exporter
EXPOSE 9615
# p2p flow
EXPOSE 30333

ENTRYPOINT ["/root/astar-collator"]
CMD ["--chain", "dev2", "--tmp", "--rpc-external", "--rpc-port", "80", "--rpc-cors", "all"]