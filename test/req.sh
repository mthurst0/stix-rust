#!/bin/bash
curl -v localhost:8080/$1 -H "Accept: application/taxii+json;version=2.1"
