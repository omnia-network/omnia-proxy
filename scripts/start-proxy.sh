#!/bin/bash

wg-quick up ./peer_proxy.conf

ifconfig

./omnia-proxy