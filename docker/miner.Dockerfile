FROM node:latest

WORKDIR /app

RUN git clone https://github.com/3Dpass/miner.git /app && \
    git checkout 2fd03a5901ce9bec52576e99d718946b6e01899a

RUN yarn install

CMD ["yarn", "miner", "--host", "node1", "--interval", "50"]
