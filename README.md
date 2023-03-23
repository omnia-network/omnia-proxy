# omnia-proxy
The omnia-proxy is a proxy server that is used to expose Gateways to the internet. It is a reverse proxy that is configured to route traffic to the appropriate Gateway based on the `X-Forward-To` header of the request.

Since it uses WireGuard under the hood, the Backend would see the request as coming from the omnia-proxy and not the actual Gateway. Because of this, the omnia-proxy will also keep track of Gateways remote IPs and add the `X-Forwarded-For` header to the request to preserve the original Gateway IP address.

## Usage
The omnia-proxy is supposed to run in Docker on a **t2.small** EC2 instance. **t1.micro** don't have enough memory to build the containers.

First, create a `.env` file in the root of the project with the following content, according to [`.env.example`](./.env.example):
```bash
# to avoid loading .env file from the host (these env vars are injected by docker compose)
ENV=production
# the port exposed by the proxy, which needs to be enabled also as ingress on the EC2 instance
PROXY_PUBLIC_PORT=80
# the public ip of the EC2 instance, currently configured on GoDaddy as
PROXY_SERVER_PUBLIC_URL=proxy.omnia-iot.com

WIREGUARD_CONTAINER_NAME=wireguard
# the url where to forward the http requests, you can use this to try
OMNIA_BACKEND_CANISTER_URL=https://swapi.dev/
# the ip assigned to wireguard container, since proxy is attached to its network
# the port specified is the port exposed by the proxy
PROXY_INTERNAL_ADDRESS=172.19.0.2:8081
```

To run the proxy, use the following command:
```bash
docker compose --profile tests up -d --build
```

After the first build and run, stop the containers with:
```bash
docker compose --profile tests down
```

And edit the `volumes/wireguard/data/wg0.conf` file to add the following line to the `[Interface]` section:
```diff
[Interface]
Address = 10.13.13.1/32
+ SaveConfig = true
PostUp = iptables -A FORWARD -i %i -j ACCEPT; iptables -A FORWARD -o %i -j ACCEPT; iptables -t nat -A POSTROUTING -o eth+ -j MASQUERADE
PostDown = iptables -D FORWARD -i %i -j ACCEPT; iptables -D FORWARD -o %i -j ACCEPT; iptables -t nat -D POSTROUTING -o eth+ -j MASQUERADE
ListenPort = 51820
PrivateKey = <generated-private-key>
```
In this way, every time the container is stopped and started, the WireGuard configuration will be saved (including newly added Peers) and the container will start with the same configuration. See also [volumes/wireguard/wg0-example.conf](./volumes/wireguard/wg0-example.conf).

To connect a Gateway, send this HTTP request to the `/register-to-vpn` endpoint of the proxy:
```bash
curl -X POST \
  http://proxy.omnia-iot.com/register-to-vpn \
  -H 'Content-Type: application/json' \
  -d '{
  "public_key": "wireguard-public-key-of-the-gateway",
}'
```

The proxy will add the Gateway to the WireGuard configuration and will return some parameters to be used by the Gateway to connect to the proxy:
```json
{
    "server_public_key": "<wireguard-server-public-key>",
    "assigned_ip": "<ip-assigned-to-the-gateway-in-the-vpn>",
    "proxy_address": "<address-of-the-proxy-to-send-requests-to-be-forwarded>"
}
```
In particular, the `proxy_address` must be the same specified in the `PROXY_INTERNAL_ADDRESS` env var.

Set the Gateway's WireGuard configuration accordingly.

From the Gateway, send HTTP requests that are supposed to be sent to the Backend to `PROXY_INTERNAL_ADDRESS`.

## Current limitations
- The proxy should dump the configuration to `volumes/proxy-rs/data/db.json` but for some reason the vpn key contains empty values.
