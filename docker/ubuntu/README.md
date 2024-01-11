# Nanvix Dockerized Tutorial

This tutorial will guide you through setting up and running Nanvix inside a Docker container.

## Prerequisites

Make sure you have Docker installed on your Ubuntu-based or derivative operating system.

## Step 1: Clone Nanvix Repository

Clone the educational version of Nanvix repository using the following command:

```bash
git clone https://github.com/nanvix/nanvix.git
cd nanvix
```
## Step 2: Build Docker Image

Build the Docker image by executing the following command:

```bash
docker build -t nanvix -f docker/ubuntu/Dockerfile .
```

## Step 2.5: Go grab a coffee and let it run!  😄
## Step 3: Run Nanvix Container

Create and run a Nanvix container with the following command:

```bash
docker run -it --rm nanvix
```
## Step 4: Interact with Nanvix Shell

Open your terminal, click on it, and press the ENTER key repeatedly until the Nanvix shell responds.

You are now set up to explore Nanvix within a Docker container! Feel free to experiment and interact with the Nanvix environment.

Note: The `--rm` flag in the `docker run` command ensures that the container is automatically removed when it exits.

Happy exploring!
