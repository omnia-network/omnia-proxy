#!/bin/bash

wg-quick up ./wg0.conf

ifconfig

./omnia-proxy