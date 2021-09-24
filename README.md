# Usage

    ./p2p [OPTIONS]

    OPTIONS:

        --bind-addr <bind-addr>     [default: 127.0.0.1]

        --connect <connect>         # connect to peer

        --period <period>

        --port <port>               [default: 8000] # bind port

# Protocol function

Each peer maintains connected peer table and every message includes hash of that table. When peer receives a message with different hash it responds back with its own peer table, then requesting peer merges received table with its own.

# Potential improvements

- peer_id authentication (peer_id is local listening address:port and is advertised to neighbor peer address:port, however backward validity is not verified on receiving end)

- separate worker and message types for peer sharing and discovery
