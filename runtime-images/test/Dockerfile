FROM alpine:3.20
EXPOSE 8080
CMD ["/bin/sh", "-c", "while true; do echo -e 'HTTP/1.1 200 OK\\r\\n\\r\\nok' | nc -l -p 8080; done"]
