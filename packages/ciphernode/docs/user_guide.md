# Running a Ciphernode

```
enclave init
```

Will setup an initial configuration

## Configuration 

Enclave is configured using a configuration file. By default this file is located under `~/.config/enclave/config.yaml`

Default values for this file might effectively look like:

```
# ~/.config/enclave/config.yaml
key_file: "{config_dir}/key"
db_file: "{data_dir}/db"
config_dir: "~/.config/enclave"
data_dir: "~/.local/share/enclave"
```

> Note if you set `config_dir` it will change the default location for both the config file and the `key_file` and if you specify `data_dir` it will change the default location for the `db_file` for example:
> If I run `enclave start --config ./some-config.yaml` where `./some-config.yaml` contains:
>
> ```
> # some-config.yaml
> config_dir: "/my/config/dir"
> ```
>
> The `enclave` binary will look for the key_file under: `/my/config/dir/key`

### Setting a relative folder as a config dir

You may set a relative folder for your config and data dirs. 

```
# /path/to/config.yaml
config_dir: ./conf
data_dir: ./data
```

Within the above config the `key_file` location will be: `/path/to/conf/key` and the `db_file` will be `/path/to/data/db`.

## Providing a registration address

_NOTE: this will likely change soon as we move to using BLS signatures for Ciphernode identification_

Ciphernodes need a registration address to identify themselves within a committee you can specify this with the `address` field within the configuration:

```
# ~/.config/enclave/config.yaml
address: "0x2546BcD3c84621e976D8185a91A922aE77ECEc30"
```

## Setting your encryption password

Your encryption password is required to encrypt sensitive data within the database such as keys and wallet private keys. You can set this key in two ways:

1. Use the command line
2. Provide a key file

## Provide your password using the commandline

```
> enclave password create

Please enter a new password:
```

Enter your chosen password.

```
Please confirm your password:
```

Enter your password again to confirm it.

```
Password sucessfully set.
```

Assuming default settings you should now be able to find your keyfile under `~/.config/enclave/key`

## Provide your password using a key file

You can use a keyfile to provide your password by creating a file under `~/.config/enclave/key` and setting the file permissions to `400`

```
mkdir -p ~/.config/enclave && read -s password && echo -n "$password" > ~/.config/enclave/key && chmod 400 ~/.config/enclave/key
```

You can change the location of your keyfile by using the `key_file` option within your configuration file:

```
# ~/.config/enclave/config.yaml
key_file: "/path/to/enclave/key"
```


