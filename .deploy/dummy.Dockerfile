# THIS IS A DUMMY IMAGE FOR TESTING
# Recreate a runtime image to test env vars without long build times
FROM debian:stable-slim
RUN apt-get update && apt-get install -y --no-install-recommends iptables ca-certificates jq dnsutils iputils-ping && \
    apt-get clean && rm -rf /var/lib/apt/lists/*

# Create non-root user
RUN useradd -m -u 1000 -s /bin/bash ciphernode

# Create necessary directories with proper permissions
RUN mkdir -p /home/ciphernode/.config/enclave \
    /home/ciphernode/.local/share/enclave \
    /run/secrets && \
    chown -R ciphernode:ciphernode /home/ciphernode /run/secrets

RUN echo "echo \"HELLO\" && sleep infinity" > /home/ciphernode/ciphernode-entrypoint.sh && \
  chmod +x /home/ciphernode/ciphernode-entrypoint.sh && \
  chown ciphernode:ciphernode /home/ciphernode/ciphernode-entrypoint.sh

# Switch to non-root user
USER ciphernode
WORKDIR /home/ciphernode

# Create the entrypoint script as root
RUN chmod +x /home/ciphernode/ciphernode-entrypoint.sh

# Environment variables for configuration
ENV CONFIG_DIR=/home/ciphernode/.config/enclave
ENV DATA_DIR=/home/ciphernode/.local/share/enclave
ENV RUST_LOG=info

# Add entrypoint script
ENTRYPOINT ["/bin/bash","/home/ciphernode/ciphernode-entrypoint.sh"]
