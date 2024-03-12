FROM alpine:3 as certs
RUN apk --update add ca-certificates

FROM scratch
COPY --from=certs /etc/ssl/certs/ca-certificates.crt /etc/ssl/certs/ca-certificates.crt
COPY ./target/x86_64-unknown-linux-musl/release/cqrs-demo* /usr/local/bin/
EXPOSE 8080
ENTRYPOINT ["cqrs-demo"]
