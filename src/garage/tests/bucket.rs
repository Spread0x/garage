use crate::common;
use aws_sdk_s3::model::BucketLocationConstraint;
use aws_sdk_s3::output::DeleteBucketOutput;

#[tokio::test]
async fn test_bucket_all() {
	let ctx = common::context();
	let bucket_name = "hello";

	{
		// Create bucket
		//@TODO check with an invalid bucket name + with an already existing bucket
		let r = ctx
			.client
			.create_bucket()
			.bucket(bucket_name)
			.send()
			.await
			.unwrap();

		//@FIXME I do not understand this result
		assert_eq!(r.location.unwrap(), "/hello");
	}
	{
		// List buckets
		let r = ctx.client.list_buckets().send().await.unwrap();

		assert_eq!(r.buckets.as_ref().unwrap().len(), 1);
		assert_eq!(
			r.buckets.unwrap().first().unwrap().name.as_ref().unwrap(),
			bucket_name
		);
	}
	{
		// Get its location
		let r = ctx
			.client
			.get_bucket_location()
			.bucket(bucket_name)
			.send()
			.await
			.unwrap();

		match r.location_constraint.unwrap() {
			BucketLocationConstraint::Unknown(v) if v.as_str() == "garage-integ-test" => (),
			_ => unreachable!("wrong region"),
		}
	}
	{
		// (Stub) check GetVersioning
		let r = ctx
			.client
			.get_bucket_versioning()
			.bucket(bucket_name)
			.send()
			.await
			.unwrap();

		assert!(r.status.is_none());
	}
	{
		// Delete bucket
		// @TODO add a check with a non-empty bucket and check failure
		let r = ctx
			.client
			.delete_bucket()
			.bucket(bucket_name)
			.send()
			.await
			.unwrap();

		assert_eq!(r, DeleteBucketOutput::builder().build());
	}
	{
		// Check bucket is deleted with List buckets
		let r = ctx.client.list_buckets().send().await.unwrap();
		assert_eq!(r.buckets.as_ref().unwrap().len(), 0);
	}
}
