FROM python@sha256:eb43ff125d8d58d7449dcba7d336c23bcac412f526d861db493b9994d8010280 AS base
ENV PYTHONUNBUFFERED=1 OMP_NUM_THREADS=1 OPENBLAS_NUM_THREADS=1 MKL_NUM_THREADS=1 TOKENIZERS_PARALLELISM=false
RUN groupadd -g 10001 lab && useradd -r -u 10001 -g lab lab && mkdir /training /models && chown lab:lab /training /models
WORKDIR /app
FROM base AS laya
COPY trainer/laya-requirements.txt ./requirements.txt
RUN pip install --no-cache-dir -r requirements.txt
COPY trainer/laya_service.py ./
ENV HF_HOME=/models
USER lab
ENTRYPOINT ["python", "/app/laya_service.py"]
FROM base AS trainer
COPY trainer/requirements.txt ./requirements.txt
RUN pip install --no-cache-dir -r requirements.txt
COPY --from=okx-tradingterminal:0.4.0 /app/okx-evaluator /app/okx-evaluator-musl
# musl static binary runs on Debian without shared runtime dependencies.
RUN cp /app/okx-evaluator-musl /app/okx-evaluator
COPY trainer/worker.py trainer/test_worker.py ./
USER lab
ENTRYPOINT ["python", "/app/worker.py"]
