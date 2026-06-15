# Deploy recipes for the standalone kaintuck-server stack (imported by the
# justfile via `import?`). Same ship-the-image flow as riparion-cms/local.just:
# build the image locally, save it to a tarball, rsync tarball + compose to the
# host, then docker load + compose up. No DB or bind mounts — state is in-memory.
#
# The host must already have the shared external `npm-network` (it does — the CMS
# stack created it) and an NPM proxy host routing kaintuck-ws.<host> → the
# container's port 4317 with WebSocket support + TLS. One-time dest prep:
#   ssh <host> 'install -d /srv/kaintuck-server'

server_tarball := "kaintuck-server.tar.gz"
server_image := "kaintuck-server:latest"
server_compose := "docker-compose.kaintuck-server.yml"
# QA + prod targets mirror riparion-cms/local.just; override with env vars.
server_host := env_var_or_default("DEPLOY_HOST", "edge.bekkey.com")
server_prod_host := env_var_or_default("PROD_HOST", "vpn.bekkey.com")
server_dest := env_var_or_default("SERVER_DEST", "/srv/kaintuck-ws")

# The reverse-proxy network NPM lives on differs per host (see the compose file).
server_qa_network := env_var_or_default("QA_PROXY_NETWORK", "proxy")
server_prod_network := env_var_or_default("PROD_PROXY_NETWORK", "npm-network")

# Build the kaintuck-server image -> ship to QA -> docker load + up.
# e.g. `just deploy-server`
deploy-server:
    docker compose -f {{server_compose}} build
    docker save {{server_image}} | gzip > {{server_tarball}}
    ssh {{server_host}} 'install -d {{server_dest}}'
    rsync -av {{server_tarball}} {{server_compose}} {{server_host}}:{{server_dest}}/
    ssh {{server_host}} 'cd {{server_dest}} && docker load < {{server_tarball}} && PROXY_NETWORK={{server_qa_network}} docker compose -f {{server_compose}} up -d && docker compose -f {{server_compose}} ps'

# Build the kaintuck-server image -> ship to prod -> docker load + up.
# e.g. `just inprod-server`
inprod-server:
    docker compose -f {{server_compose}} build
    docker save {{server_image}} | gzip > {{server_tarball}}
    ssh {{server_prod_host}} 'install -d {{server_dest}}'
    rsync -av {{server_tarball}} {{server_compose}} {{server_prod_host}}:{{server_dest}}/
    ssh {{server_prod_host}} 'cd {{server_dest}} && docker load < {{server_tarball}} && PROXY_NETWORK={{server_prod_network}} docker compose -f {{server_compose}} up -d && docker compose -f {{server_compose}} ps'
