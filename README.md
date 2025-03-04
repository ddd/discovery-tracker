
# discovery-tracker

Tracker for changes in [Google Discovery Documents](https://developers.google.com/discovery) with optional Discord webhook support written in Rust.

**Demo:**
[tracker.brute.cat](https://tracker.brute.cat)

![Local API](./static/webhook.png)


### Installation via Docker (linux/wsl)

Install docker via the instructions at https://docs.docker.com/engine/install/

```
git clone https://github.com/ddd/discovery-tracker
cd discovery-tracker
mkdir -p data/{changes,storage}
```

Modify the config as necessary

```
vim config.yaml
```

Start the container

```
sudo docker compose up -d --build
```

The local API will be available at http://localhost:3000
