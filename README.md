# Greeting app in Rust
This is a simple sample app implementing a service for receiving greetings and storing them in a database. 
It furthers distributes greetings via Kafka topic with Debezium

# Enable offline build for SQLX
```
cargo sqlx prepare'  
cargo build
```
The component is a sample app in RUST for creating a service for receiving greetings
It implements an API for receiving and listing greetings. 
The service publishes events from new greetings downstream for consumers. 

Setting up kafka and debezium on Docker. 

#activat debezium connector 
```
curl -i -X POST -H "Accept:application/json" -H "Content-Type:application/json" localhost:8083/connectors/ --data "@debezium.json"
```
# deleting debezium connector
```
curl -i -X DELETE localhost:8083/connectors/greeting-connector
```


Minikube
configure zookeper for minikube based on article:https://gsfl3101.medium.com/kafka-raft-kraft-cluster-configuration-from-dev-to-prod-part-1-8a844fabf804
```
kubectl apply -f kubernetes/kafka-zookeeper.yaml
kubectl apply -f kubernetes/kafka.yaml

kubectl delete -n default deployment kafka-deployment-1
kubectl delete -n default deployment kafka-deployment-2
kubectl delete -n default service kafka-service
```

# Building docker image
In order to build a dockerimage, the following init was done. 
```
docker init
docker-compose up --build
```

The basic setup didn't build correctly due to cross compilation from macos to alpine

The following sections steps through problems and solutions  
## Updating versions, due to errors bulding
After troubleshooting building docker image, 
Update rust version: 
```
rustup update
```
## Cross compiliation of rdkafka dependencies 
Building the docker image failed with some errors refering to cross compliation of
rdkafka-sys dependencies. 

Added to the "# Install host build dependencies."  
```
#RUN apk add --no-cache clang lld musl-dev git librdkafka-dev g++ make
```

## Building app with SQLX 
In order to build application with sqlx, the macros used in code need to validate SQL
Tryding to use the updated query-cache from development
Set 
```
ENV SQLX_OFFLINE true
```
Mount generated sqlx cache to build 
```
--mount=type=bind,source=.sqlx,target=.sqlx \
```

After some work making Dockerfile build a successfull image, the container refused to run. 
In order to investiage, I installed the utility strace in order to trace the linux environment
for clues. 
By reading output, it was pretty easy to relate errors to missing files and external resources not accessible. 


# Deploying image to minikube from local development
In order to deploy a locally built image from local docker registry follow steps for macos:


```
TAG="0.23" 
docker build -q -t "arnecdn/greeting-rust:${TAG}" . &&
mkdir -p .docker && docker image save -o .docker/greeting-rust.tar "arnecdn/greeting-rust:${TAG}" &&
minikube image load .docker/greeting-rust.tar
```
From minikube: https://minikube.sigs.k8s.io/docs/handbook/pushing/
```
Remember to turn off the imagePullPolicy:Always 
(use imagePullPolicy:IfNotPresent or imagePullPolicy:Never) 
in your yaml file. Otherwise Kubernetes won’t use your locally build image and 
it will pull from the network.
```

Further deployment includes installing additional services on Minikube as Kafka, Kafka-Connect and Postgres. 
The specs are stored in ./kubernetes/ folder and can be deployed as a unit.
Make sure the image of rust-docker is available for minikube. See section over. 
```
kubectl apply -f kubernetes/kafka.yaml

kubectl apply -f kubernetes/greeting-rust.yaml

kubectl exec -it kafka-0 -- bash
kafka-topics --create --topic greetings --replication-factor 1 --bootstrap-server kafka-0:9092
kafka-topics --create --topic greetings --replication-factor 3 --partitions 10 --bootstrap-server kafka-0:9092
kafka-topics --create --topic greetings --partitions 10 --bootstrap-server kafka-0:9092

kafka-topics --bootstrap-server kafka-0:9092 --topic greetings --describe
```