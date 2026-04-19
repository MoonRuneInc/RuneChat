FROM alpine:3.23

# 70 is the standard uid/gid for "postgres" in Alpine
RUN set -eux; \
    addgroup -g 70 -S postgres; \
    adduser -u 70 -S -D -G postgres -H -h /var/lib/postgresql -s /bin/sh postgres; \
    install --verbose --directory --owner postgres --group postgres --mode 1777 /var/lib/postgresql

RUN apk add --no-cache \
    bash \
    su-exec \
    postgresql16 \
    postgresql16-client \
    postgresql16-contrib \
    tzdata

# Ensure PostgreSQL listens on all interfaces (like official image)
RUN sed -i "s/^#listen_addresses = 'localhost'/listen_addresses = '*'/" /usr/share/postgresql16/postgresql.conf.sample

ENV LANG=en_US.utf8
ENV PG_MAJOR=16
ENV PGDATA=/var/lib/postgresql/data

RUN mkdir /docker-entrypoint-initdb.d
COPY docker-entrypoint.sh /usr/local/bin/
RUN chmod +x /usr/local/bin/docker-entrypoint.sh

RUN mkdir -p "$PGDATA" && chown -R postgres:postgres "$PGDATA" && chmod 700 "$PGDATA"
VOLUME /var/lib/postgresql/data

ENTRYPOINT ["docker-entrypoint.sh"]

EXPOSE 5432
CMD ["postgres"]
