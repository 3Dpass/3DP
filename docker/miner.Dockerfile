FROM node:latest

WORKDIR /app

RUN git clone https://github.com/3Dpass/miner.git /app && \
    git checkout 698c885de5ff11568b4aab1ca819391e0e05819c

RUN yarn install

CMD ["yarn", "miner", "--host", "node1", "--interval", "50"]
