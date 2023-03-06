FROM node:latest

WORKDIR /app

COPY ./miner-libs/ .
COPY ./package.json .
COPY ./pnpm-lock.yaml .
COPY ./miner.js .

RUN corepack enable && \
    corepack prepare pnpm@latest --activate && \
    pnpm install

CMD ["pnpm", "miner", "--host", "node"]
