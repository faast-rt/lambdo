# Lambdo

<div align="center">
  <img src="https://user-images.githubusercontent.com/50552672/207000761-65516609-5c16-4263-92bd-f6f01eaaac84.png" alt="Lambdo logo" style="width:200px;"/>
  <br>
  <h1>Lambdo</h1>
  <p>A Serverless runtime in Rust </p>
</div>

## What is Lambdo?

Lambdo is a Serverless runtime in Rust, which is inspired by [AWS Lambda](https://aws.amazon.com/lambda/). It's aim is to provide a simple and fast way to run the code on [Polycode](https://polycode.do-2021.fr) platform.

## How to use it?

The first step is to compile all the necessary stuff to run the code. To do so, you need :

- A Linux kernel (>= 4.14) complied without initramfs bundled but with the lzma compression support.
- Build the initramfs with the `initramfs` folder in this repository including the sdk of the language you want to use and a working init script.
- Create a configuration file for lambdo see the example in `examples/node/config.yaml`.

You **MUST** have to install KVM on your machine to run the runtime.

To start the runtime, you need to run the `lambdo` binary with the configuration file as argument :

```bash
$ lambdo --config /path/to/config.yaml
```

or use `Docker` image :

```bash
$ docker run -it --privileged -p 3000:3000 \
  -v /path/to/config:/etc/lambdo/config.yaml \
  -v /path/to/initramfs:/var/lib/lambdo/initramfs \
  -v /path/to/kernel:/var/lib/lambdo/kernel/vmlinux.bin \
  dopolytech2021/lambdo
```

If you want to run `lambdo` for a test, you can use the `docker-compose` file that will use the `examples/node` folder as configuration folder.

```bash
$ docker-compose up

# then you can run the following command to call the runtime
$ curl --location 'http://127.0.0.1:3000/run' \
  --header 'Content-Type: application/json' \
  --data '{
      "language": "NODE",
      "version": "1.0.0",
      "input": "",
      "code": [{
          "filename": "main.js",
          "content": "console.log('\''Hello World!'\'')"
      }]
  }'
```

## Contributing

Pull requests are welcome. For major changes, please open an issue first to discuss what you would like to change.

Please make sure to update tests as appropriate and follow Angular instruction for commit name ([here](https://github.com/angular/angular/blob/master/CONTRIBUTING.md)).
