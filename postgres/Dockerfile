FROM postgres@sha256:77f585114c32fbca283dc835b0596f4e52b51b4c6662d7810b2f4084f60a1873
# Compose already starts as postgres: the root privilege-switch helper is unused.
RUN apk add --no-cache --upgrade nghttp2-libs=1.70.0-r0 && rm /usr/local/bin/gosu
USER postgres
