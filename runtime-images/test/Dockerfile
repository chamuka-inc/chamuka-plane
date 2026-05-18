FROM alpine:3.20
RUN apk add --no-cache tini
EXPOSE 8080
ENTRYPOINT ["tini", "--"]
CMD ["/bin/sh", "-c", "while true; do echo -e 'HTTP/1.1 200 OK\\r\\n\\r\\nok' | nc -l -p 8080; done"]
