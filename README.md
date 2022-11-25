# Service Quotas

- [Service Quotas](#service-quotas)
  - [Quick Start](#quick-start)


A CLI to calculate utilization of AWS service quotas, using [CloudWatch service quota usage metrics](https://docs.aws.amazon.com/AmazonCloudWatch/latest/monitoring/CloudWatch-Service-Quota-Integration.html).

The CLI will discover all service quotas via the [list-services](https://docs.aws.amazon.com/servicequotas/2019-06-24/apireference/API_ListServices.html) and [list-service-quotas](https://docs.aws.amazon.com/servicequotas/2019-06-24/apireference/API_ListServiceQuotas.html) API. If the service quota supports cloudwatch metrics and the `SERVICE_QUOTA()` metric maths function then it will query for the utilization percentage of the service quota, and report any utilization of service quotas that breaches the threshold (75% as default).

## Quick Start

```
docker run robpickerill/service-quotas -h

docker run -e AWS_ACCESS_KEY_ID -e AWS_SECRET_ACCESS_KEY -e AWS_SESSION_TOKEN robpickerill/service-quotas -r us-east-1 -r us-west-2
```

Note: AWS credentials are lifted from the environment variables.