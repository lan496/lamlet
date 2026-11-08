FROM debian:bookworm-slim@sha256:734a72b3d6142a71993b4bc580ca01d8ca687f77a2c422d96a961d3376b179bc AS build
RUN apt-get update && apt-get install -y --no-install-recommends ninja-build g++ python3 python3-venv && rm -rf /var/lib/apt/lists/*
RUN python3 -m venv /build-tools && /build-tools/bin/pip install --no-cache-dir cmake==3.31.6
ENV PATH=/build-tools/bin:$PATH
COPY source/ /source/
RUN cmake -S /source/cmake -B /build -G Ninja -DBUILD_MPI=off -DBUILD_OMP=off -DBUILD_SHARED_LIBS=on -DPKG_MANYBODY=on -DCMAKE_BUILD_TYPE=Release && cmake --build /build --parallel 2

FROM debian:bookworm-slim@sha256:734a72b3d6142a71993b4bc580ca01d8ca687f77a2c422d96a961d3376b179bc AS runtime
RUN apt-get update && apt-get install -y --no-install-recommends python3 libstdc++6 ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=build /build/lmp /opt/lammps/bin/lmp
COPY --from=build /build/liblammps.so* /opt/lammps/python/lammps/
COPY --from=build /source/python/lammps/ /opt/lammps/python/lammps/
ENV PYTHONPATH=/opt/lammps/python LD_LIBRARY_PATH=/opt/lammps/python/lammps PATH=/opt/lammps/bin:$PATH
RUN useradd --uid 1000 --create-home agent && mkdir -p /app/submission /logs/verifier /logs/artifacts && chown -R agent:agent /app /logs
WORKDIR /app
USER agent

FROM runtime AS verifier
USER root
COPY p3_oracle.py /opt/lamlet-bench/oracle
RUN chmod 0555 /opt/lamlet-bench/oracle
USER agent
