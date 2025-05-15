#!/bin/bash

if [ ! -z "$1" ]
then
  name=$1
else
  echo -n "Enter name of the service: "
  read name
fi

if [ ! -z "$2" ]
then
  name=$2
else
  echo -n "Enter the service binding address: "
  read bind_addr 
fi



mkdir -p /opt/pink-lady/$name/scripts

cd /opt/pink-lady/$name/

cat <<EOF > .env
RUST_LOG=info
PL_NAME=$name
PL_SCRIPT_FOLDER=/opt/pink-lady/$name/scripts/
PL_BIND=$bind_addr
EOF



