module.exports = {
  apps: [
    {
      name: "miner",
      script: "miner.js",
      args: "--host 127.0.0.1 --port 9833 --interval 10",
      instances: 4,
      autorestart: true,
      watch: false,
      max_memory_restart: "1G",
      env: {
        NODE_ENV: "development",
      },
      env_production: {
        NODE_ENV: "production",
      },
    },
  ],
};
