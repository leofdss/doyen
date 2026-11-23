FROM quay.io/toolbx/arch-toolbox:latest

RUN pacman-key --init && pacman-key --populate archlinux

RUN pacman -Syu --noconfirm --needed \
      sudo \
      git \
      base-devel \
      python \
      fnm \
      #
      rust \
      llvm \
      # VSCode Plugin
      rust-src \
      libsecret \
      nss \
      #
      fish \
      starship \
      #
      && pacman -Scc --noconfirm \
      && rm -rf /var/cache/pacman/pkg/* /var/lib/pacman/sync/*

RUN cargo install cargo-llvm-cov

ENV DONT_PROMPT_WSL_INSTALL=1
