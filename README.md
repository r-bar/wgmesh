# wgmesh: The Wireguard Mesh Network Daemon

wgmesh is a lightweight program for creating and managing mesh networks using
the wireguard VPN.


# Design

wgmesh will connect to the given host and attempt to contact a wgmesh daemon on
that host. If it does it will get a list of other known hosts and add them to
the connection. As new hosts join the network other hosts will be notified for
the new connection and be updated.

## Propagating Events

Events like connection and disconnection will be forwarded to other hosts. Each
event carries a UUIDv1. Hosts will not forward events they have already seen,
and newer UUIDs will override conflicting older ones.

## Service Endpoints

wgmesh hosts a small web server to propagate information to the other hosts.

### POST `/connect`

New hosts will hit this endpoint when they first try to contact other hosts.
This endpoint accepts a payload describing the connecting host including public
key and available network interfaces. The endpoint will respond with a list of
other hosts. Returns the same payload as `/discover`.

### GET `/discover`

Get a list of known hosts. Similar to connect, but does not prompt the remote
host to alert other nodes about your connection.

### GET `/ping`

Check connection to remote host.

### POST `/disconnect`

Gracefully disconnect from the network. The receiving host will alert other
hosts of the disconnection.
