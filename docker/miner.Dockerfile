FROM node:latest

WORKDIR /app

COPY ./package.json .
COPY ./pnpm-lock.yaml .

RUN corepack enable && \
    corepack prepare pnpm@latest --activate && \
    pnpm install

COPY ./miner-libs/ ./miner-libs/
COPY ./miner.js .

CMD ["pnpm", "miner", "--host", "node"]
