# Stage 1: Build the Vue application
FROM node:26-alpine AS builder
WORKDIR /app

COPY package*.json ./
RUN npm ci

COPY . .
RUN npm run build

# Stage 2: Serve static files with high-performance static-web-server
FROM docker.io/joseluisq/static-web-server:2-alpine AS runtime

COPY --from=builder /app/dist /public

ENV SERVER_PORT=8787
ENV SERVER_ROOT=/public
ENV SERVER_FALLBACK_PAGE=/public/index.html
ENV SERVER_LOG_LEVEL=info
ENV SERVER_COMPRESSION=true

EXPOSE 8787
CMD ["static-web-server"]
