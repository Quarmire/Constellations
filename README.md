# Constellations, a library for building distributed knowledge

# Running
```
cargo build
```

Make application directory.
```
sudo mkdir /var/lib/constellations
```

Run celestia-daemon.
```
sudo ./target/debug/celestiad -c config.json
```

Open spaceport named 'test'.
```
curl -v http://localhost:9494/v0/open?name=test
```

Run pub application (must be run and have real-time mode enabled before subscribers can view updates)
```
sudo ./target/debug/pub
```

Run sub application (it may be required that the application is started, exited, then started again to fetch the text blocks from the other devices; this is a bug in the software.)
```
sudo ./target/debug/sub
```