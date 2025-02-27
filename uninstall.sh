#/bin/sh

# remove config files
sudo rm -r ~/.config/edock

# remove binary
echo "Removing binary from /usr/local/bin ..."
sudo rm /usr/local/bin/edock
