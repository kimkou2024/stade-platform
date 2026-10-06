# Frontend image — SvelteKit SSR (adapter-node).
FROM node:22-slim AS build
WORKDIR /app
COPY package.json package-lock.json* ./
RUN npm install
COPY . .
RUN npm run build

FROM node:22-slim
WORKDIR /app
COPY --from=build /app/build ./build
COPY --from=build /app/package.json ./package.json
COPY --from=build /app/node_modules ./node_modules
EXPOSE 3000
ENV PORT=3000
# PUBLIC_API_BASE is injected at runtime via the reverse proxy / env.
CMD ["node", "build"]
