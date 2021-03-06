FROM golang:1.15.2 as go-builder
RUN mkdir /build
ADD ./src/go /build/
WORKDIR /build
# Following flags are passed to ld (external linker):
# * -w to decrease binary size by not including debug info
# * -extldflags "-static" to build static binaries
RUN CGO_ENABLED=0 GOOS=linux \
    go build -a \
        -ldflags '-w -extldflags "-static"' \
        -o k8s-api-test cmd/k8s-api-test/main.go
RUN CGO_ENABLED=0 GOOS=linux \
    go build -a \
        -ldflags '-w -extldflags "-static"' \
        -o stress-tester cmd/stress-tester/main.go

FROM rust:alpine3.12 as rust-builder
COPY ./src/rust/receiver-mock /build
WORKDIR /build
RUN apk update && apk upgrade && apk add g++
RUN cargo build --release

FROM alpine:3.12
ENV HELM_VERSION="3.2.4"
ENV YQ_VERSION="3.2.1"
ENV KUBECTL_VERSION="v1.18.10"
RUN set -ex \
    && apk update \
    && apk upgrade \
    && apk add --no-cache \
        bash \
        busybox-extras \
        curl \
        libc6-compat \
        openssl \
        net-tools \
        vim \
        jq \
    && curl https://get.helm.sh/helm-v${HELM_VERSION}-linux-amd64.tar.gz | tar -xzO linux-amd64/helm > /usr/local/bin/helm \
    && chmod +x /usr/local/bin/helm \
    && curl -LJ https://github.com/mikefarah/yq/releases/download/${YQ_VERSION}/yq_linux_amd64 -o /usr/bin/yq \
    && chmod +x /usr/bin/yq \
    && curl -LJ https://storage.googleapis.com/kubernetes-release/release/${KUBECTL_VERSION}/bin/linux/amd64/kubectl -o /usr/bin/kubectl \
    && chmod +x /usr/bin/kubectl

COPY \
    ./src/ssh/motd \
    ./src/ssh/profile \
    /etc/

COPY \
    ./src/commands/check \
    ./src/commands/fix-log-symlinks \
    ./src/commands/tools-usage \
    ./src/commands/template \
    ./src/commands/template-dependency \
    /usr/bin/

COPY ./src/commands/template-prometheus-mixin \
    /usr/local/template-prometheus-mixin
RUN ln -s /usr/local/template-prometheus-mixin/template-prometheus-mixin /usr/bin

COPY --from=go-builder \
    /build/k8s-api-test \
    /build/stress-tester \
    /usr/bin/

COPY --from=rust-builder \
    /build/target/release/receiver-mock \
    /usr/bin/

CMD ["/usr/bin/tools-usage"]
