ARG CHECKER_IMAGE
ARG BASELINE_IMAGE
FROM ${CHECKER_IMAGE} AS previous_checker
FROM ${BASELINE_IMAGE} AS common
USER root
COPY repair_instrument.py /opt/repair_instrument.py
RUN chmod 0555 /opt/repair_instrument.py && ln -s /opt/repair_instrument.py /usr/local/bin/lmp
USER agent

FROM common AS checker
USER root
COPY --from=previous_checker /usr/local/bin/lamlet /opt/lamlet/lamlet
RUN ln -s /opt/repair_instrument.py /usr/local/bin/lamlet
USER agent
