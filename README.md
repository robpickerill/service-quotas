# Service Quotas

- [Service Quotas](#service-quotas)
  - [Quick Start](#quick-start)
  - [Notifications](#notifications)
    - [Pagerduty](#pagerduty)
  - [IAM Permissions](#iam-permissions)


A CLI to calculate utilization of AWS service quotas, using [CloudWatch service quota usage metrics](https://docs.aws.amazon.com/AmazonCloudWatch/latest/monitoring/CloudWatch-Service-Quota-Integration.html). 

Discovery of service quotas via the [list-services](https://docs.aws.amazon.com/servicequotas/2019-06-24/apireference/API_ListServices.html) and [list-service-quotas](https://docs.aws.amazon.com/servicequotas/2019-06-24/apireference/API_ListServiceQuotas.html) API. If the service quota supports cloudwatch metrics and the `SERVICE_QUOTA()` metric maths function then it will query for the utilization percentage of the service quota. This provides the programmatic glue between the AWS Service Quota list-service-quota API `UsageMetric` and the Cloudwatch metric maths query to obtain the utilization of the service quota.

Additionally, any breached quotas (whereby the utilization is greater than the threshold) can be passed to incident response systems, like Pagerduty.

## Quick Start

```bash
docker run robpickerill/service-quotas -h

# help output
docker run -e AWS_ACCESS_KEY_ID -e AWS_SECRET_ACCESS_KEY -e AWS_SESSION_TOKEN robpickerill/service-quotas -h

# display supported quotas for multiple regions
docker run -e AWS_ACCESS_KEY_ID -e AWS_SECRET_ACCESS_KEY -e AWS_SESSION_TOKEN robpickerill/service-quotas list-quotas -r eu-west-1 eu-west-2

# run over multiple regions, ignoring the quota code: L-E9E9831D
docker run -e AWS_ACCESS_KEY_ID -e AWS_SECRET_ACCESS_KEY -e AWS_SESSION_TOKEN robpickerill/service-quotas utilization -r us-east-1 us-east-2 us-west-2 -i L-E9E9831D
```

Note: AWS credentials are lifted from the environment variables.


## Notifications

Any service quotas that exceed the threshold will create notifications. At the time of writing, Pagerduty notifications are supported.

### Pagerduty

In order to enable pagerduty notifications, ensure the service routing key for the [EventsV2 API](https://developer.pagerduty.com/docs/ZG9jOjExMDI5NTgw-events-api-v2-overview) is available as an environment variable:

```bash
export PAGERDUTY_ROUTING_KEY=key_here
```

## IAM Permissions

Permissions must be granted for the following actions:

- cloudwatch:GetMetricData
- servicequotas:ListServices
- servicequotas:ListServiceQuotas

An example IAM policy is provided as:

```json
{
    "Version": "2012-10-17",
    "Statement": [
        {
            "Sid": "AllowCloudWatch",
            "Action": [
                "cloudwatch:GetMetricData"
            ],
            "Effect": "Allow",
            "Resource": "*"
        },
        {
            "Sid": "AllowSeviceQuotas",
            "Action": [
                "servicequotas:ListServices",
                "servicequotas:ListServiceQuotas"
            ],
            "Effect": "Allow",
            "Resource": "*"
        }
    ]
}
```
