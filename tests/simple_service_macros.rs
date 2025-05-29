// Test for the service and action macros
//
// This test demonstrates how to use the service and action macros
// to create a simple service with actions.

use anyhow::{anyhow, Result};
use futures::lock::Mutex;
use runar_common::types::ArcValueType;
use runar_macros::{action, publish, service, subscribe};
use runar_node::config::{LogLevel, LoggingConfig};
use runar_node::services::{EventContext, RequestContext};
use runar_node::Node;
use runar_node::NodeConfig;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, sync::Arc};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct MyData {
    id: i32,
    text_field: String,
    number_field: i32,
    boolean_field: bool,
    float_field: f64,
    vector_field: Vec<i32>,
    map_field: HashMap<String, i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct User {
    id: i32,
    name: String,
    email: String,
    age: i32,
}

// Define a simple math service
pub struct TestService {
    store: Arc<Mutex<HashMap<String, ArcValueType>>>,
}

// Implement Clone manually for TestMathService
impl Clone for TestService {
    fn clone(&self) -> Self {
        Self {
            store: self.store.clone(),
        }
    }
}

#[service(
    name = "Test Service Name",
    path = "math",
    description = "Test Service Description",
    version = "0.0.1"
)]
impl TestService {
    #[action]
    async fn get_user(&self, id: i32, ctx: &RequestContext) -> Result<User> {
        let user = User {
            id,
            name: "John Doe".to_string(),
            email: "john.doe@example.com".to_string(),
            age: 30,
        };
        Ok(user)
    }

    //the publish macro will do a ctx.publish("my_data_auto", ArcValueType::from_struct(action_result.clone())).await?;
    //it will publish the result of the action o the path (full or relative) same ruleas as action, subscribe macros in termos fo topic rules.,
    #[publish(path = "my_data_auto")]
    #[action(path = "my_data")]
    async fn get_my_data(&self, id: i32, ctx: &RequestContext) -> Result<MyData> {
        // Log using the context
        ctx.debug(format!("get_my_data id: {}", id));

        let total_res = ctx
            .request(
                "math/add",
                Some(ArcValueType::new_map(HashMap::from([
                    ("a".to_string(), 1000.0),
                    ("b".to_string(), 500.0),
                ]))),
            )
            .await?;
        let mut data = total_res.unwrap();
        let total = data.as_type::<f64>()?;

        // Return the result
        let data = MyData {
            id,
            text_field: "test".to_string(),
            number_field: id,
            boolean_field: true,
            float_field: total,
            vector_field: vec![1, 2, 3],
            map_field: HashMap::new(),
        };
        ctx.publish(
            "my_data_changed",
            Some(ArcValueType::from_struct(data.clone())),
        )
        .await?;
        ctx.publish("age_changed", Some(ArcValueType::new_primitive(25)))
            .await?;
        Ok(data)
    }

    #[subscribe(path = "math/my_data_auto")]
    async fn on_my_data_auto(&self, data: MyData, ctx: &EventContext) -> Result<()> {
        ctx.debug(format!(
            "my_data_auto was an event published using the publish macro ->: {}",
            data.text_field
        ));

        let mut lock = self.store.lock().await;
        let existing = lock.get("my_data_auto");
        if let Some(existing) = existing {
            let mut existing = existing.clone();
            let mut existing = existing.as_type::<Vec<MyData>>().unwrap();
            existing.push(data.clone());
            lock.insert("my_data_auto".to_string(), ArcValueType::new_list(existing));
        } else {
            lock.insert(
                "my_data_auto".to_string(),
                ArcValueType::new_list(vec![data.clone()]),
            );
        }

        Ok(())
    }

    #[subscribe(path = "math/added")]
    async fn on_added(&self, total: f64, ctx: &EventContext) -> Result<()> {
        ctx.debug(format!("on_added: {}", total));

        let mut lock = self.store.lock().await;
        let existing = lock.get("added");
        if let Some(existing) = existing {
            let mut existing = existing.clone();
            let mut existing = existing.as_type::<Vec<f64>>().unwrap();
            existing.push(total);
            lock.insert("added".to_string(), ArcValueType::new_list(existing));
        } else {
            lock.insert("added".to_string(), ArcValueType::new_list(vec![total]));
        }

        Ok(())
    }

    #[subscribe(path = "math/my_data_changed")]
    async fn on_my_data_changed(&self, data: MyData, ctx: &EventContext) -> Result<()> {
        ctx.debug(format!("my_data_changed: {}", data.text_field));

        let mut lock = self.store.lock().await;
        let existing = lock.get("my_data_changed");
        if let Some(existing) = existing {
            let mut existing = existing.clone();
            let mut existing = existing.as_type::<Vec<MyData>>().unwrap();
            existing.push(data.clone());
            lock.insert(
                "my_data_changed".to_string(),
                ArcValueType::new_list(existing),
            );
        } else {
            lock.insert(
                "my_data_changed".to_string(),
                ArcValueType::new_list(vec![data.clone()]),
            );
        }

        Ok(())
    }

    #[subscribe(path = "math/age_changed")]
    async fn on_age_changed(&self, new_age: i32, ctx: &EventContext) -> Result<()> {
        ctx.debug(format!("age_changed: {}", new_age));

        let mut lock = self.store.lock().await;
        let existing = lock.get("age_changed");
        if let Some(existing) = existing {
            let mut existing = existing.clone();
            let mut existing = existing.as_type::<Vec<i32>>().unwrap();
            existing.push(new_age);
            lock.insert("age_changed".to_string(), ArcValueType::new_list(existing));
        } else {
            lock.insert(
                "age_changed".to_string(),
                ArcValueType::new_list(vec![new_age]),
            );
        }

        Ok(())
    }

    // Define an action using the action macro
    #[publish(path = "added")]
    #[action]
    async fn add(&self, a: f64, b: f64, ctx: &RequestContext) -> Result<f64> {
        // Log using the context
        ctx.debug(format!("Adding {} + {}", a, b));
        // Return the result
        Ok(a + b)
    }

    // Define another action
    #[action]
    async fn subtract(&self, a: f64, b: f64, ctx: &RequestContext) -> Result<f64> {
        // Log using the context
        ctx.debug(format!("Subtracting {} - {}", a, b));

        // Return the result
        Ok(a - b)
    }

    // Define an action with a custom name
    #[action("multiply_numbers")]
    async fn multiply(&self, a: f64, b: f64, ctx: &RequestContext) -> Result<f64> {
        // Log using the context
        ctx.debug(format!("Multiplying {} * {}", a, b));

        // Return the result
        Ok(a * b)
    }

    // Define an action that can fail
    #[action]
    async fn divide(&self, a: f64, b: f64, ctx: &RequestContext) -> Result<f64> {
        // Log using the context
        ctx.debug(format!("Dividing {} / {}", a, b));

        // Check for division by zero
        if b == 0.0 {
            ctx.error("Division by zero".to_string());
            return Err(anyhow::anyhow!("Division by zero"));
        }

        // Return the result
        Ok(a / b)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use runar_node::config::LogLevel;
    use runar_node::config::LoggingConfig;
    use runar_node::Node;
    use runar_node::NodeConfig;

    #[tokio::test]
    async fn test_math_service() {
        //set log to debug
        let logging_config = LoggingConfig::new().with_default_level(LogLevel::Debug);

        // Create a node with a test network ID
        let mut config =
            NodeConfig::new("test-node", "test_network").with_logging_config(logging_config);
        // Disable networking
        config.network_config = None;
        let mut node = Node::new(config).await.unwrap();

        let store = Arc::new(Mutex::new(HashMap::new()));

        // Create a test math service
        let service = TestService {
            store: store.clone(),
        };

        // Add the service to the node
        node.add_service(service).await.unwrap();

        // Start the node to initialize all services
        node.start().await.unwrap();

        // Create parameters for the add action
        let mut map = std::collections::HashMap::new();
        map.insert("a".to_string(), 10.0);
        map.insert("b".to_string(), 5.0);
        let params = ArcValueType::new_map(map);

        // Call the add action
        let response = node.request("math/add", Some(params)).await.unwrap();

        // Verify the response
        assert_eq!(response.unwrap().as_type::<f64>().unwrap(), 15.0);

        // Make a request to the subtract action
        let mut map = std::collections::HashMap::new();
        map.insert("a".to_string(), 10.0);
        map.insert("b".to_string(), 5.0);
        let params = ArcValueType::new_map(map);

        let response = node.request("math/subtract", Some(params)).await.unwrap();

        // Verify the response
        assert_eq!(response.unwrap().as_type::<f64>().unwrap(), 5.0);

        // Make a request to the multiply action (with custom name)
        let mut map = std::collections::HashMap::new();
        map.insert("a".to_string(), 5.0);
        map.insert("b".to_string(), 3.0);
        let params = ArcValueType::new_map(map);

        let response = node
            .request("math/multiply_numbers", Some(params))
            .await
            .unwrap();

        // Verify the response
        assert_eq!(response.unwrap().as_type::<f64>().unwrap(), 15.0);

        // Make a request to the divide action with valid parameters
        let mut map = std::collections::HashMap::new();
        map.insert("a".to_string(), 6.0);
        map.insert("b".to_string(), 3.0);
        let params = ArcValueType::new_map(map);

        let response = node.request("math/divide", Some(params)).await.unwrap();

        // Verify the response
        assert_eq!(response.unwrap().as_type::<f64>().unwrap(), 2.0);

        // Make a request to the divide action with invalid parameters (division by zero)
        let mut map = std::collections::HashMap::new();
        map.insert("a".to_string(), 6.0);
        map.insert("b".to_string(), 0.0);
        let params = ArcValueType::new_map(map);

        let response = node.request("math/divide", Some(params)).await;

        // Verify the error response
        assert!(response
            .unwrap_err()
            .to_string()
            .contains("Division by zero"));

        // Make a request to the get_user action
        let mut map = std::collections::HashMap::new();
        map.insert("id".to_string(), 42);
        let params = ArcValueType::new_map(map);

        let response = node.request("math/get_user", Some(params)).await.unwrap();

        // Verify the response
        let user = response.unwrap().as_type::<User>().unwrap();
        assert_eq!(user.id, 42);
        assert_eq!(user.name, "John Doe");

        // Make a request to the get_my_data action
        let mut map = std::collections::HashMap::new();
        map.insert("id".to_string(), 1);
        let params = ArcValueType::new_map(map);

        let response = node.request("math/my_data", Some(params)).await.unwrap();

        // Verify the response
        let my_data = response.unwrap().as_type::<MyData>().unwrap();
        assert_eq!(
            my_data,
            MyData {
                id: 1,
                text_field: "test".to_string(),
                number_field: 1,
                boolean_field: true,
                float_field: 1500.0,
                vector_field: vec![1, 2, 3],
                map_field: HashMap::new(),
            }
        );

        // Let's assert all the events stored in our store
        let store = store.lock().await;

        // Check if my_data_auto events were stored correctly as a vector
        if let Some(my_data_arc) = store.get("my_data_auto") {
            let mut my_data_arc = my_data_arc.clone(); // Clone to get ownership
            let my_data_vec = my_data_arc.as_list_ref::<MyData>().unwrap();
            assert!(
                !my_data_vec.is_empty(),
                "Expected at least one my_data_auto event"
            );
            assert_eq!(
                my_data_vec[0], my_data,
                "The first my_data_auto event doesn't match expected data"
            );
            println!("my_data_auto events count: {}", my_data_vec.len());
        } else {
            panic!("Expected 'my_data_auto' key in store, but it wasn't found");
        }

        // Check for added events
        if let Some(added_arc) = store.get("added") {
            let mut added_arc = added_arc.clone();
            let added_vec = added_arc.as_list_ref::<f64>().unwrap();
            assert!(!added_vec.is_empty(), "Expected at least one added event");
            assert_eq!(added_vec[0], 15.0, "Expected first added value to be 15.0"); // 10.0 + 5.0
            assert_eq!(
                added_vec[1], 1500.0,
                "Expected second added value to be 1500.0"
            ); // 1000.0 + 500.0
            assert_eq!(added_vec.len(), 2, "Expected two added events");
            println!("added events count: {}", added_vec.len());
        } else {
            panic!("Expected 'added' key in store, but it wasn't found");
        }

        // Check for my_data_changed events
        if let Some(changed_arc) = store.get("my_data_changed") {
            let mut changed_arc = changed_arc.clone();
            let changed_vec = changed_arc.as_list_ref::<MyData>().unwrap();
            assert!(
                !changed_vec.is_empty(),
                "Expected at least one my_data_changed event"
            );
            assert_eq!(
                changed_vec[0].id, my_data.id,
                "Expected first my_data_changed.id to match"
            );
            println!("my_data_changed events count: {}", changed_vec.len());
        } else {
            panic!("Expected 'my_data_changed' key in store, but it wasn't found");
        }

        // Check for age_changed events
        if let Some(age_arc) = store.get("age_changed") {
            let mut age_arc = age_arc.clone();
            let age_vec = age_arc.as_list_ref::<i32>().unwrap();
            assert!(
                !age_vec.is_empty(),
                "Expected at least one age_changed event"
            );
            assert_eq!(age_vec[0], 25, "Expected first age_changed value to be 25");
            assert_eq!(age_vec.len(), 1, "Expected one age_changed event");
            println!("age_changed events count: {}", age_vec.len());
        } else {
            panic!("Expected 'age_changed' key in store, but it wasn't found");
        }
        //make sure type were added properly to the serializer
        let serializer = node.serializer.read().await;
        let arc_value = ArcValueType::from_struct(my_data.clone());
        let bytes = serializer.serialize_value(&arc_value).unwrap();

        // Create an Arc<[u8]> directly from the Vec<u8>
        let arc_bytes = Arc::from(bytes);

        let mut deserialized = serializer.deserialize_value(arc_bytes).unwrap();
        let deserialized_my_data = deserialized.as_type::<MyData>().unwrap();

        assert_eq!(deserialized_my_data, my_data);

        //make sure type were added properly to the serializer
        let user = User {
            id: 42,
            name: "John Doe".to_string(),
            email: "john.doe@example.com".to_string(),
            age: 30,
        };
        let arc_value = ArcValueType::from_struct(user.clone());
        let bytes = serializer.serialize_value(&arc_value).unwrap();

        // Create an Arc<[u8]> directly from the Vec<u8>
        let arc_bytes = Arc::from(bytes);

        let mut deserialized = serializer.deserialize_value(arc_bytes).unwrap();
        let deserialized_user = deserialized.as_type::<User>().unwrap();

        assert_eq!(deserialized_user, user);
    }
}
