FROM swiftnav/rust:2020.12.08

ARG WORKSPACE=/home/jenkins

RUN echo "Create jenkins user"... \
    && useradd -u 1001 -ms /bin/bash -G sudo,staff jenkins \
    && echo '%sudo ALL=(ALL) NOPASSWD:ALL' >> /etc/sudoers
USER jenkins

RUN rustup default nightly
