# omnia-proxy
The omnia-proxy is a proxy server that is used to expose Gateways to the internet. It is a reverse proxy that is configured to route traffic to the appropriate Gateway based on the `X-Forward-To-Peer` header of the request. This header **must** contain the **UUID** assigned by the proxy to the Gateway. See [Endpoints](#endpoints) for more details.

A `X-Forward-To-Port` header can be used to specify to which port on the peer the request should be forwarded (by default, it is set to `8888`, the default Gateway WoT Servient port).

Since it uses WireGuard under the hood, the Backend would see the request as coming from the omnia-proxy and not the actual Gateway. Because of this, the omnia-proxy will also keep track of Gateways remote IPs and add the `X-Proxied-For` header to the request to preserve the original Gateway IP address.

## Usage
The omnia-proxy is supposed to run in Docker on a **t2.small** EC2 instance. **t1.micro** don't have enough memory to build the containers.

First, create a `.env` file in the root of the project with the following content, according to [`.env.example`](./.env.example):
```bash
# to avoid loading .env file from the host (these env vars are injected by docker compose)
ENV=production
# the proxy port to expose outside, which needs to be enabled also as ingress on the EC2 instance
# typically, if you enable HTTPS, this should be 443, otherwise 80
PROXY_PUBLIC_PORT=80
# the public ip of the EC2 instance, currently configured on GoDaddy as
PROXY_SERVER_PUBLIC_URL=proxy.omnia-iot.com
# the HTTP server listens to this port. You cannot change this port unless you enable HTTPS
PROXY_PORT=8081
# there are also some env variables to enable HTTPS, see below.
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

## HTTPS support
By default, the proxy is configured to use HTTP.

To **add** (**and only add**, since the proxy needs to listen on HTTP inside the VPN) HTTPS support, you have to create an HTTPS certificate and put it in the volume mounted in `proxy-rs` container. You can use [certbot (running in Docker)](https://eff-certbot.readthedocs.io/en/stable/install.html#alternative-1-docker) to create a certificate for the domain you want to use:
```bash
docker run -it --rm \
    -v <absolute-path-to-this-folder>/volumes/proxy-rs/certs:/etc/letsencrypt \
    -p 80:80 \
    certbot/certbot certonly --standalone -d <your-domain>
```
Follow the steps in the interactive shell.

> This will spawn a temporary HTTP server on port `80` to enable Let's Encrypt to verify the domain. Make sure no other process is listening on that port.

You should end up with a `volumes` folder structure like:
```
volumes
├── proxy-rs
│   ├── certs
│   │   ├── live
│   │   │   └── <your-domain>
│   │   │       ├── fullchain.pem
│   │   │       ├── privkey.pem
│   │   │       └── README
│   │   ├── archive
│   │   ├── renewal
...
```
You can then set the environment variables in the `.env` file to enable HTTPS support:
```bash
# override the port set for HTTP for public facing communications. 443 is the only port that can be used for HTTPS
PROXY_PORT=443
ENABLE_HTTPS=true
HTTPS_CERT_PATH=/proxy/certs/live/<your-domain>/fullchain.pem
HTTPS_KEY_PATH=/proxy/certs/live/<your-domain>/privkey.pem
```
and start the proxy with Docker Compose as usual.

## Endpoints
### `/register-to-vpn`
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
    "assigned_id": "<uuid-assigned-to-the-gateway-by-the-proxy>",
    "proxy_address": "<address-of-the-proxy-to-send-requests-to-be-forwarded>"
}
```
In particular, the `proxy_address` is the same specified in the `PROXY_INTERNAL_ADDRESS` env var.

Set the Gateway's WireGuard configuration accordingly. An example of the configuration can be:
```
[Interface]
Address = <assigned_ip>/32
ListenPort = 51820
PrivateKey = <gateway_private_key>

[Peer]
PublicKey = <server_public_key>
AllowedIPs = 0.0.0.0/0
Endpoint = <PROXY_SERVER_PUBLIC_URL>:51820

```
A useful tool to generate WireGuard configurations is [WireGuard Tools](https://www.wireguardconfig.com/).

From the Gateway, send HTTP requests that are supposed to be sent to the Backend to `PROXY_INTERNAL_ADDRESS`, adding a `X-Destination-Url` header to tell the proxy where to forward the request, e.g. the Backend canister URL or the Application canister URL.

### `/peer-info`
**Once a Peer is connected to the VPN**, it get its own information by sending a GET request to the `/peer-info` endpoint of the proxy:
```bash
curl -X GET \
  http://<PROXY_INTERNAL_ADDRESS>/peer-info \
  -H 'X-Forward-To-Peer: <gateway-uuid>'
```
It will receive a response like:
```json
{
    "id": "<gateway-uuid>",
    "internal_ip": "<gateway-ip-in-the-vpn>",
    "public_ip": "<gateway-public-ip>",
    "public_key": "<gateway-public-key>",
    "proxy_address": "<proxy-internal-address>"
}
```

### `/health-check`
This endpoint just returns a `200 OK` response.

## Current limitations
- The proxy **doesn't** remove unused/disconnected peers from the WireGuard configuration and from the local database.
- Every time the proxy received a request, it <u>reads WireGuard status from docker command line</u> to get the updated public IP of the peers. This is not efficient and should be improved.
- communications between peers and proxy inside the VPN are on HTTP, _not HTTPS_.

## Improvements
We could use [localtunnel server](https://github.com/localtunnel/server) to achieve the same result, without WireGuard and with a simpler setup.