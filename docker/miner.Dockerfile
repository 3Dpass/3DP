FROM node:latest

WORKDIR /app

RUN git clone https://github.com/3Dpass/miner.git /app && \
    git checkout ccdc5a5b5e6d09431390c387f86f9ce498ab376f

RUN yarn install

CMD ["yarn", "miner", "--host", "node1", "--interval", "50"]
