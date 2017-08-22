# compilenv is a pre-compiled docker image containing all binaries dependancies for nanvix
FROM turbolay/nanvix:compilenv

MAINTAINER turbolay "https://github.com/turbolay"

# Setting up ENV variable for checkout on updated branch
ARG BRANCH=local
ENV BRANCH ${BRANCH}

# Pull & checkout to last updated branch of nanvix
RUN git clone --branch=$BRANCH https://github.com/nanvix/nanvix.git /root/nanvix

# Command below would be a better implementation than ENV variable but it's always returning "master" on Docker
# RUN git checkout $(git branch --sort=-committerdate | grep '*' | awk -F " " '{print $2}')

# Make port 4567 listenable for Travis-CI
EXPOSE 4567
