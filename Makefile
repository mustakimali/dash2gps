VERSION?=local
IMAGE_NAME?=mustakimali/dash2gps

.PHONY: docker-build
docker-build:
	docker build -t $(IMAGE_NAME):$(VERSION) -f Dockerfile .

.PHONY: docker-push
docker-push:
	make docker-build && \
	docker tag mustakimali/dash2gps:local mustakimali/dash2gps:latest && \
	docker push mustakimali/dash2gps
