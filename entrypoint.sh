#!/bin/sh

# Generate credentials.csv file for AWS CLI
printf "User Name,Access key ID,Secret access key\ndefault,$AWS_ACCESS_KEY_ID,$AWS_SECRET_ACCESS_KEY" > credentials.csv

# Import the credentials.csv file into AWS CLI
aws configure import --csv file://credentials.csv

# Validate aws identity
aws sts get-caller-identity > /dev/null

exec "$@"