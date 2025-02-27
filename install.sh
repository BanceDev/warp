#/bin/sh

# build the project
cargo build --release

# copy the binary to the directory
echo "Copying binary to /usr/local/bin ..."
sudo cp ./target/release/edock /usr/local/bin

# create the config directory
sudo mkdir -p ~/.config/edock

# copy style.css to the config directory
sudo cp ./res/style.css ~/.config/edock

# copy the config file to the config directory
sudo cp ./res/config ~/.config/edock
