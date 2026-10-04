FROM node@sha256:0b36e8c136b94cd4fcf02188228e76c31ad5872eef3fec8cbd2eee500cfd9e80
WORKDIR /app
COPY package*.json ./
RUN apk upgrade --no-cache && npm ci --ignore-scripts && rm -rf /usr/local/lib/node_modules/npm /opt/yarn-* /usr/local/bin/npm /usr/local/bin/npx /usr/local/bin/yarn /usr/local/bin/yarnpkg && mkdir /jobs && chown 10001:10001 /jobs
COPY run.mjs broker.mjs tests.mjs ./
USER 10001:10001
CMD ["node","/app/broker.mjs"]
